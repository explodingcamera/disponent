use proc_macro2::TokenStream;
use quote::quote;
use syn::{Result, spanned::Spanned};

pub fn forward_to_variant(
    inherent: bool,
    inline: bool,
    enum_def: &syn::ItemEnum,
    trait_def: &syn::ItemTrait,
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

    let variants: Vec<_> = enum_def
        .variants
        .iter()
        .filter_map(|v| match &v.fields {
            syn::Fields::Unnamed(f) if f.unnamed.len() == 1 => {
                Some((&v.ident, &f.unnamed.first()?.ty, &v.attrs))
            }
            _ => None,
        })
        .collect();

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
    let trait_ident = &trait_def.ident;

    let variant_bounds: Vec<_> = if !trait_def.generics.params.is_empty() {
        {
            variants
                .iter()
                .map(|(_, ty, _)| quote! { #ty: #trait_ident #trait_ty_generics })
                .collect()
        }
    } else {
        Default::default()
    };

    let trait_generics = (inherent && !trait_def.generics.params.is_empty()).then_some((
        &trait_def.generics,
        trait_where_clause,
        &variant_bounds,
    ));

    let methods: Vec<_> = trait_def
        .items
        .iter()
        .filter_map(|item| match item {
            syn::TraitItem::Fn(m) => Some(generate_method(
                inherent,
                inline,
                m,
                enum_ident,
                trait_ident,
                &variants,
                trait_generics,
            )),
            _ => None,
        })
        .collect::<Result<_>>()?;

    Ok(if inherent {
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
            impl #enum_impl_generics #trait_impl_generics #trait_ident #trait_ty_generics for #enum_ident #enum_ty_generics #where_clause { #(#methods)* }
        }
    })
}

fn generate_method(
    inherent: bool,
    inline: bool,
    method: &syn::TraitItemFn,
    enum_ident: &syn::Ident,
    trait_ident: &syn::Ident,
    variants: &[(&syn::Ident, &syn::Type, &Vec<syn::Attribute>)],
    trait_generics: Option<(&syn::Generics, Option<&syn::WhereClause>, &Vec<TokenStream>)>,
) -> Result<TokenStream> {
    let mut sig = method.sig.clone();

    // Check for unsupported self types like `self: Arc<Self>`
    let self_ty = match sig.inputs.first() {
        Some(syn::FnArg::Typed(p)) => Some(&p.ty),
        Some(syn::FnArg::Receiver(r)) => Some(&r.ty),
        None => None,
    };
    if let Some(ty) = self_ty.filter(|t| is_wrapped_self(t)) {
        return Err(syn::Error::new(
            ty.span(),
            "Arbitrary self types like `Arc<Self>` or `Box<Self>` are not supported. Use `self`, `&self`, or `&mut self` instead.",
        ));
    }

    let (is_impl_future, ret) = extract_future_output(&sig.output);
    sig.output = ret;
    let is_async = is_impl_future || sig.asyncness.is_some();
    sig.asyncness = is_async.then(|| syn::Token![async](proc_macro2::Span::call_site()));

    if let Some((trait_gens, trait_where, variant_bounds)) = trait_generics {
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

    // Replace Self with enum ident in non-receiver arguments and return type
    for p in sig.inputs.iter_mut().skip(1).filter_map(|a| match a {
        syn::FnArg::Typed(p) => Some(p),
        _ => None,
    }) {
        replace_self(&mut p.ty, enum_ident);
    }
    if let syn::ReturnType::Type(_, t) = &mut sig.output {
        replace_self(t, enum_ident);
    }

    // Check for reserved parameter names
    let inner = quote::format_ident!("__disponent_inner");
    for p in sig.inputs.iter().skip(1).filter_map(|a| match a {
        syn::FnArg::Typed(p) => Some(p),
        _ => None,
    }) {
        if let syn::Pat::Ident(pat) = &*p.pat
            && pat.ident == inner
        {
            return Err(syn::Error::new(
                pat.ident.span(),
                "Parameter name `__disponent_inner` is reserved. Use a different name.",
            ));
        }
    }

    let attrs = method.attrs.iter().filter(|a| is_attr_allowed(a, true));
    let vis = inherent.then(|| quote! { pub });
    let args: Vec<_> = sig
        .inputs
        .iter()
        .skip(1)
        .filter_map(|a| match a {
            syn::FnArg::Typed(p) => Some(&p.pat),
            _ => None,
        })
        .collect();

    let method_ident = &sig.ident;
    let arms = variants.iter().map(|(v, _, attrs)| {
        let variant_attrs = attrs.iter().filter(|a| is_attr_allowed(a, false));
        let call = quote! { #trait_ident::#method_ident(#inner, #(#args),*) };
        let call = is_async.then(|| quote! { #call.await }).unwrap_or(call);
        quote! { #(#variant_attrs)* #enum_ident::#v(#inner) => #call, }
    });

    let inline_attr = inline.then(|| quote! { #[inline] });

    Ok(quote! { #(#attrs)* #inline_attr #vis #sig { match self { #(#arms)* } } })
}

fn generic_param_name(p: &syn::GenericParam) -> &syn::Ident {
    match p {
        syn::GenericParam::Type(t) => &t.ident,
        syn::GenericParam::Lifetime(l) => &l.lifetime.ident,
        syn::GenericParam::Const(c) => &c.ident,
    }
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

fn replace_self(ty: &mut syn::Type, ident: &syn::Ident) {
    let syn::Type::Path(p) = ty else { return };
    if p.path.segments.len() == 1 && p.path.segments[0].ident == "Self" {
        *ty = syn::Type::Path(syn::TypePath {
            qself: None,
            path: ident.clone().into(),
        });
        return;
    }

    for seg in &mut p.path.segments {
        if let syn::PathArguments::AngleBracketed(args) = &mut seg.arguments {
            for arg in &mut args.args {
                match arg {
                    syn::GenericArgument::Type(t) => replace_self(t, ident),
                    syn::GenericArgument::AssocType(at) => replace_self(&mut at.ty, ident),
                    _ => {}
                }
            }
        }
    }
}
