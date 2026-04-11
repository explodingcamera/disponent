use proc_macro2::TokenStream;
use quote::quote;
use syn::{Result, spanned::Spanned};

use crate::InherentConfig;

struct ForwardCtx<'a> {
    inherent_vis: Option<&'a syn::Visibility>,
    inline: bool,
    enum_ident: &'a syn::Ident,
    trait_path: &'a syn::Path,
    trait_ty_generics: &'a syn::TypeGenerics<'a>,
    variants: &'a [(&'a syn::Ident, &'a syn::Type, &'a Vec<syn::Attribute>)],
    fallback_variant: Option<(&'a syn::Ident, &'a syn::Type)>,
    trait_generics: Option<(
        &'a syn::Generics,
        Option<&'a syn::WhereClause>,
        &'a Vec<TokenStream>,
    )>,
}

pub fn forward_to_variant(
    inherent: Option<&InherentConfig>,
    inline: bool,
    enum_def: &syn::ItemEnum,
    trait_def: &syn::ItemTrait,
    remote_path: Option<&syn::Path>,
) -> Result<TokenStream> {
    if !enum_def.generics.params.is_empty() && !trait_def.generics.params.is_empty() {
        return Err(syn::Error::new(
            enum_def.generics.span(),
            "Cannot combine enum and trait generics",
        ));
    }

    for item in &trait_def.items {
        if let Some(msg) = match item {
            syn::TraitItem::Type(_) => Some("Associated types not supported"),
            syn::TraitItem::Const(_) => Some("Associated constants not supported"),
            _ => None,
        } {
            return Err(syn::Error::new(item.span(), msg));
        }
    }

    let variants: Vec<(&syn::Ident, &syn::Type, &Vec<syn::Attribute>)> = enum_def
        .variants
        .iter()
        .filter_map(|v| match &v.fields {
            syn::Fields::Unnamed(f) if f.unnamed.len() == 1 => {
                Some((&v.ident, &f.unnamed.first()?.ty, &v.attrs))
            }
            _ => None,
        })
        .collect();

    let fallback_variants: Vec<_> = variants
        .iter()
        .filter(|(_, _, attrs)| attrs.iter().any(is_fallback_attr))
        .collect();
    if fallback_variants.len() > 1 {
        return Err(syn::Error::new(
            enum_def.ident.span(),
            "Only one enum variant can be marked with #[fallback]",
        ));
    }
    let fallback_variant = fallback_variants
        .first()
        .map(|(ident, ty, _)| (*ident, *ty));

    if variants.len() != enum_def.variants.len() {
        return Err(syn::Error::new(
            enum_def.span(),
            "All variants must be newtype with one field",
        ));
    }

    let (enum_impl_generics, enum_ty_generics, enum_where_clause) =
        enum_def.generics.split_for_impl();
    let (trait_impl_generics, trait_ty_generics, trait_where_clause) =
        trait_def.generics.split_for_impl();
    let enum_ident = &enum_def.ident;
    let local_trait_path: syn::Path = trait_def.ident.clone().into();
    let trait_path = remote_path.unwrap_or(&local_trait_path);

    let variant_bounds: Vec<_> = if !trait_def.generics.params.is_empty() {
        variants
            .iter()
            .map(|(_, ty, _)| quote! { #ty: #trait_path #trait_ty_generics })
            .collect()
    } else {
        Default::default()
    };

    let trait_generics = (inherent.is_some() && !trait_def.generics.params.is_empty()).then_some((
        &trait_def.generics,
        trait_where_clause,
        &variant_bounds,
    ));

    let inherent_vis = inherent.map(|i| match i {
        InherentConfig::Inherit => &enum_def.vis,
        InherentConfig::Explicit(vis) => vis,
    });

    let ctx = ForwardCtx {
        inherent_vis,
        inline,
        enum_ident,
        trait_path,
        trait_ty_generics: &trait_ty_generics,
        variants: &variants,
        fallback_variant,
        trait_generics,
    };

    let methods: Vec<_> = trait_def
        .items
        .iter()
        .filter_map(|item| match item {
            syn::TraitItem::Fn(m) => Some(generate_method(m, &ctx)),
            _ => None,
        })
        .collect::<Result<_>>()?;

    Ok(if inherent.is_some() {
        let where_clause = build_where_clause(enum_where_clause, None, &[]);
        quote! {
            #[automatically_derived]
            impl #enum_impl_generics #enum_ident #enum_ty_generics #where_clause { #(#methods)* }
        }
    } else {
        let where_clause =
            build_where_clause(enum_where_clause, trait_where_clause, &variant_bounds);
        quote! {
            #[automatically_derived]
            impl #enum_impl_generics #trait_impl_generics #trait_path #trait_ty_generics for #enum_ident #enum_ty_generics #where_clause { #(#methods)* }
        }
    })
}

fn generate_method(method: &syn::TraitItemFn, ctx: &ForwardCtx<'_>) -> Result<TokenStream> {
    let ForwardCtx {
        inherent_vis: inherent,
        inline,
        enum_ident,
        trait_path,
        trait_ty_generics,
        variants,
        fallback_variant,
        trait_generics,
    } = ctx;

    let mut sig = method.sig.clone();

    let has_receiver = sig.receiver().is_some();

    // Check for unsupported self types like `self: Arc<Self>`
    if has_receiver
        && let Some(receiver) = sig.receiver()
        && is_wrapped_self(&receiver.ty)
    {
        return Err(syn::Error::new(
            receiver.ty.span(),
            "Arbitrary self types like `Arc<Self>` or `Box<Self>` are not supported. Use `self`, `&self`, or `&mut self` instead.",
        ));
    }

    if !has_receiver && fallback_variant.is_none() {
        return Err(syn::Error::new(
            sig.ident.span(),
            "Methods without a receiver require a #[fallback] enum variant",
        ));
    }

    let (is_impl_future, ret) = extract_future_output(&sig.output);
    sig.output = ret;
    let is_async = is_impl_future || sig.asyncness.is_some();
    sig.asyncness = is_async.then(|| syn::Token![async](proc_macro2::Span::call_site()));

    if let Some((trait_gens, trait_where, variant_bounds)) = *trait_generics {
        // Check for generic name clashes
        let trait_names: std::collections::HashSet<_> = trait_gens
            .params
            .iter()
            .map(|p| generic_param_name(p).to_string())
            .collect();

        for param in &sig.generics.params {
            let ident = generic_param_name(param);
            if trait_names.contains(&ident.to_string()) {
                return Err(syn::Error::new(
                    ident.span(),
                    format!(
                        "Generic parameter `{}` conflicts with trait generic parameter. Use a different name.",
                        ident
                    ),
                ));
            }
        }

        sig.generics.params = trait_gens
            .params
            .iter()
            .chain(&sig.generics.params)
            .cloned()
            .collect();

        sig.generics.where_clause = build_where_clause(
            sig.generics.where_clause.as_ref(),
            trait_where,
            variant_bounds,
        );
    }

    let enum_self_ty: syn::Type = syn::parse_quote!(#enum_ident);

    // Replace Self with enum ident in non-receiver arguments and return type
    for p in typed_inputs_mut(&mut sig, has_receiver) {
        replace_self_with(&mut p.ty, &enum_self_ty);
    }
    if let syn::ReturnType::Type(_, t) = &mut sig.output {
        replace_self_with(t, &enum_self_ty);
    }

    if !has_receiver
        && let Some((_, fallback_ty)) = fallback_variant
        && let Some(where_clause) = &mut sig.generics.where_clause
    {
        for predicate in &mut where_clause.predicates {
            if let syn::WherePredicate::Type(ty_pred) = predicate {
                replace_self_with(&mut ty_pred.bounded_ty, fallback_ty);
                for bound in &mut ty_pred.bounds {
                    if let syn::TypeParamBound::Trait(trait_bound) = bound {
                        replace_self_in_path(&mut trait_bound.path, fallback_ty);
                    }
                }
            }
        }
    }

    let returns_self = returns_bare_self(&method.sig.output);

    if has_receiver {
        let has_self_in_non_receiver_args = typed_inputs(&method.sig, true)
            .map(|pat| &*pat.ty)
            .any(type_contains_self);

        if has_self_in_non_receiver_args {
            return Err(syn::Error::new(
                method.sig.span(),
                "Methods with a receiver cannot use `Self` in non-receiver parameters",
            ));
        }

        if return_type_contains_self(&method.sig.output) && !returns_self {
            return Err(syn::Error::new(
                method.sig.output.span(),
                "Methods with a receiver only support bare `-> Self` return types",
            ));
        }
    }

    // Check for reserved parameter names
    let inner = quote::format_ident!("__disponent_inner");
    for p in typed_inputs(&sig, has_receiver) {
        if let syn::Pat::Ident(pat) = &*p.pat {
            if pat.ident == inner {
                return Err(syn::Error::new(
                    pat.ident.span(),
                    "Parameter name `__disponent_inner` is reserved. Use a different name.",
                ));
            }
        }
    }

    let attrs = method.attrs.iter().filter(|a| is_attr_allowed(a, true));
    let vis = inherent.map(|v| quote! { #v });
    let args: Vec<_> = typed_inputs(&sig, has_receiver).map(|p| &p.pat).collect();

    let method_ident = &sig.ident;

    let body = if has_receiver {
        let arms = variants.iter().map(|(v, _, attrs)| {
            let variant_attrs = attrs.iter().filter(|a| is_attr_allowed(a, false));
            let call = quote! { #trait_path::#method_ident(#inner, #(#args),*) };
            let call = is_async.then(|| quote! { #call.await }).unwrap_or(call);
            let call = if returns_self {
                quote! { #enum_ident::#v(#call) }
            } else {
                call
            };
            quote! { #(#variant_attrs)* #enum_ident::#v(#inner) => #call, }
        });
        quote! { match self { #(#arms)* } }
    } else {
        let (fallback_ident, fallback_ty) = fallback_variant.expect("validated above");
        let call = quote! {
            <#fallback_ty as #trait_path #trait_ty_generics>::#method_ident(#(#args),*)
        };
        let call = is_async.then(|| quote! { #call.await }).unwrap_or(call);
        if returns_self {
            quote! { #enum_ident::#fallback_ident(#call) }
        } else {
            call
        }
    };

    let inline_attr = (*inline).then(|| quote! { #[inline] });

    Ok(quote! { #(#attrs)* #inline_attr #vis #sig { #body } })
}

fn generic_param_name(p: &syn::GenericParam) -> &syn::Ident {
    match p {
        syn::GenericParam::Type(t) => &t.ident,
        syn::GenericParam::Lifetime(l) => &l.lifetime.ident,
        syn::GenericParam::Const(c) => &c.ident,
    }
}

fn typed_inputs(
    sig: &syn::Signature,
    has_receiver: bool,
) -> impl Iterator<Item = &syn::PatType> {
    sig.inputs
        .iter()
        .skip(usize::from(has_receiver))
        .filter_map(|arg| match arg {
            syn::FnArg::Typed(pat) => Some(pat),
            _ => None,
        })
}

fn typed_inputs_mut(
    sig: &mut syn::Signature,
    has_receiver: bool,
) -> impl Iterator<Item = &mut syn::PatType> {
    sig.inputs
        .iter_mut()
        .skip(usize::from(has_receiver))
        .filter_map(|arg| match arg {
            syn::FnArg::Typed(pat) => Some(pat),
            _ => None,
        })
}

fn is_attr_allowed(attr: &syn::Attribute, include_doc: bool) -> bool {
    let allowed = if include_doc {
        &["cfg", "cfg_attr", "doc"] as &[_]
    } else {
        &["cfg", "cfg_attr"] as &[_]
    };
    attr.path()
        .segments
        .last()
        .is_some_and(|s| allowed.contains(&s.ident.to_string().as_str()))
}

fn extract_future_output(output: &syn::ReturnType) -> (bool, syn::ReturnType) {
    let syn::ReturnType::Type(_, ty) = output else {
        return (false, output.clone());
    };

    let syn::Type::ImplTrait(impl_trait) = ty.as_ref() else {
        return (false, output.clone());
    };

    let output_ty = impl_trait
        .bounds
        .iter()
        .filter_map(|b| match b {
            syn::TypeParamBound::Trait(t) => Some(t),
            _ => None,
        })
        .find_map(|t| t.path.segments.last().filter(|s| s.ident == "Future"))
        .and_then(|s| match &s.arguments {
            syn::PathArguments::AngleBracketed(args) => Some(args),
            _ => None,
        })
        .and_then(|args| {
            args.args.iter().find_map(|arg| match arg {
                syn::GenericArgument::AssocType(at) if at.ident == "Output" => Some(at.ty.clone()),
                _ => None,
            })
        });

    output_ty
        .map(|t| (true, syn::ReturnType::Type(Default::default(), Box::new(t))))
        .unwrap_or((false, output.clone()))
}

fn build_where_clause(
    enum_where: Option<&syn::WhereClause>,
    trait_where: Option<&syn::WhereClause>,
    variant_bounds: &[TokenStream],
) -> Option<syn::WhereClause> {
    if enum_where.is_none() && trait_where.is_none() && variant_bounds.is_empty() {
        return None;
    }

    let mut combined = enum_where.cloned().unwrap_or_else(|| syn::WhereClause {
        where_token: syn::Token![where](proc_macro2::Span::call_site()),
        predicates: Default::default(),
    });

    if let Some(tw) = trait_where {
        combined.predicates.extend(tw.predicates.clone());
    }

    for bound in variant_bounds {
        combined.predicates.push(syn::parse_quote!(#bound));
    }

    Some(combined)
}

fn is_wrapped_self(ty: &syn::Type) -> bool {
    let syn::Type::Path(p) = ty else { return false };
    let (Some(segment), true) = (p.path.segments.last(), p.path.segments.len() == 1) else {
        return false;
    };
    let syn::PathArguments::AngleBracketed(args) = &segment.arguments else {
        return false;
    };
    let (Some(arg), true) = (args.args.first(), args.args.len() == 1) else {
        return false;
    };
    let syn::GenericArgument::Type(syn::Type::Path(inner)) = arg else {
        return false;
    };
    inner.path.segments.len() == 1 && inner.path.segments[0].ident == "Self"
}

fn replace_self_with(ty: &mut syn::Type, replacement: &syn::Type) {
    let syn::Type::Path(p) = ty else { return };
    if p.path.segments.len() == 1 && p.path.segments[0].ident == "Self" {
        *ty = replacement.clone();
        return;
    }

    for seg in &mut p.path.segments {
        if let syn::PathArguments::AngleBracketed(args) = &mut seg.arguments {
            for arg in &mut args.args {
                match arg {
                    syn::GenericArgument::Type(t) => replace_self_with(t, replacement),
                    syn::GenericArgument::AssocType(at) => {
                        replace_self_with(&mut at.ty, replacement)
                    }
                    _ => {}
                }
            }
        }
    }
}

fn replace_self_in_path(path: &mut syn::Path, replacement: &syn::Type) {
    for seg in &mut path.segments {
        if let syn::PathArguments::AngleBracketed(args) = &mut seg.arguments {
            for arg in &mut args.args {
                match arg {
                    syn::GenericArgument::Type(t) => replace_self_with(t, replacement),
                    syn::GenericArgument::AssocType(at) => {
                        replace_self_with(&mut at.ty, replacement)
                    }
                    _ => {}
                }
            }
        }
    }
}

fn returns_bare_self(output: &syn::ReturnType) -> bool {
    let syn::ReturnType::Type(_, ty) = output else {
        return false;
    };
    let syn::Type::Path(type_path) = ty.as_ref() else {
        return false;
    };
    type_path.qself.is_none()
        && type_path.path.segments.len() == 1
        && type_path.path.segments[0].ident == "Self"
}

fn is_fallback_attr(attr: &syn::Attribute) -> bool {
    attr.path().is_ident("fallback")
}

fn return_type_contains_self(output: &syn::ReturnType) -> bool {
    match output {
        syn::ReturnType::Default => false,
        syn::ReturnType::Type(_, ty) => type_contains_self(ty),
    }
}

fn type_contains_self(ty: &syn::Type) -> bool {
    match ty {
        syn::Type::Path(type_path) => {
            if type_path.qself.is_none()
                && type_path.path.segments.len() == 1
                && type_path.path.segments[0].ident == "Self"
            {
                return true;
            }

            type_path.path.segments.iter().any(|segment| {
                let syn::PathArguments::AngleBracketed(args) = &segment.arguments else {
                    return false;
                };
                args.args.iter().any(|arg| match arg {
                    syn::GenericArgument::Type(t) => type_contains_self(t),
                    syn::GenericArgument::AssocType(at) => type_contains_self(&at.ty),
                    _ => false,
                })
            })
        }
        syn::Type::Reference(r) => type_contains_self(&r.elem),
        syn::Type::Tuple(t) => t.elems.iter().any(type_contains_self),
        syn::Type::Paren(p) => type_contains_self(&p.elem),
        syn::Type::Group(g) => type_contains_self(&g.elem),
        syn::Type::Array(a) => type_contains_self(&a.elem),
        syn::Type::Slice(s) => type_contains_self(&s.elem),
        syn::Type::Ptr(p) => type_contains_self(&p.elem),
        _ => false,
    }
}
