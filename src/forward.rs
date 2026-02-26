use proc_macro2::TokenStream;
use quote::quote;
use syn::{spanned::Spanned, Result};

pub fn forward_to_variant(
    inherent: bool,
    inline: bool,
    enum_def: &syn::ItemEnum,
    trait_def: &syn::ItemTrait,
) -> Result<TokenStream> {
    (!enum_def.generics.params.is_empty() && !trait_def.generics.params.is_empty())
        .then(|| {
            syn::Error::new(
                enum_def.generics.span(),
                "Cannot combine enum and trait generics",
            )
        })
        .map_or(Ok(()), Err)?;

    trait_def.items.iter().try_for_each(|item| match item {
        syn::TraitItem::Type(_) => Err(syn::Error::new(
            item.span(),
            "Associated types not supported",
        )),
        syn::TraitItem::Const(_) => Err(syn::Error::new(
            item.span(),
            "Associated constants not supported",
        )),
        _ => Ok(()),
    })?;

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

    (variants.len() == enum_def.variants.len())
        .then_some(())
        .ok_or_else(|| {
            syn::Error::new(
                enum_def.span(),
                "All variants must be newtype with one field",
            )
        })?;

    let generics = if enum_def.generics.params.is_empty() {
        &trait_def.generics
    } else {
        &enum_def.generics
    };
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
    let enum_ident = &enum_def.ident;
    let trait_ident = &trait_def.ident;

    let methods = trait_def.items.iter().filter_map(|item| match item {
        syn::TraitItem::Fn(m) => Some(generate_method(
            inherent,
            inline,
            m,
            enum_ident,
            trait_ident,
            &variants,
        )),
        _ => None,
    });

    Ok(if inherent {
        quote! {
            #[automatically_derived]
            impl #impl_generics #enum_ident #ty_generics #where_clause { #(#methods)* }
        }
    } else {
        let trait_ty = (!trait_def.generics.params.is_empty()).then(|| {
            let (_, t, _) = trait_def.generics.split_for_impl();
            quote! { #t }
        });
        quote! {
            #[automatically_derived]
            impl #impl_generics #trait_ident #trait_ty for #enum_ident #ty_generics #where_clause { #(#methods)* }
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
) -> TokenStream {
    let mut sig = method.sig.clone();
    // Flatten `impl Future<Output = T>` to `async fn -> T` so all variants return the same future type
    let (is_async, ret) = extract_future_output(&sig.output);
    sig.output = ret;
    sig.asyncness = is_async.then(|| syn::Token![async](proc_macro2::Span::call_site()));

    sig.inputs
        .iter_mut()
        .filter_map(|i| match i {
            syn::FnArg::Typed(p) => Some(p),
            _ => None,
        })
        .for_each(|p| replace_self(&mut p.ty, enum_ident));
    if let syn::ReturnType::Type(_, t) = &mut sig.output {
        replace_self(t, enum_ident);
    }

    let attrs = method.attrs.iter().filter(|a| {
        a.path()
            .segments
            .last()
            .is_some_and(|s| matches!(s.ident.to_string().as_str(), "cfg" | "cfg_attr" | "doc"))
    });
    let vis = inherent.then(|| quote! { pub });

    let inner = quote::format_ident!("inner");
    // `self` by value has no `reference`, unlike `&self` and `&mut self`
    let receiver = match sig.inputs.first() {
        Some(syn::FnArg::Receiver(r)) if r.reference.is_some() && r.mutability.is_some() => {
            quote! { &mut self }
        }
        Some(syn::FnArg::Receiver(r)) if r.reference.is_some() => quote! { &self },
        Some(syn::FnArg::Receiver(_)) => quote! { self },
        _ => quote! { self },
    };

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
    // When matching on `&self`, the pattern `Enum::Variant(inner)` binds `inner: &T`
    let arms = variants.iter().map(|(v, _, attrs)| {
        let variant_attrs = attrs.iter().filter(|a| {
            a.path()
                .segments
                .last()
                .is_some_and(|s| matches!(s.ident.to_string().as_str(), "cfg" | "cfg_attr"))
        });
        let call = quote! { #trait_ident::#method_ident(#inner, #(#args),*) };
        let call = if is_async {
            quote! { #call.await }
        } else {
            call
        };
        quote! { #(#variant_attrs)* #enum_ident::#v(#inner) => #call, }
    });

    let inline_attr = inline.then(|| quote! { #[inline] });

    quote! { #(#attrs)* #inline_attr #vis #sig { match #receiver { #(#arms)* } } }
}

/// Extracts the `Output` type from `impl Future<Output = T>`, returning `(true, T)`.
/// Returns `(false, original)` if not a Future.
fn extract_future_output(output: &syn::ReturnType) -> (bool, syn::ReturnType) {
    let ty = match output {
        syn::ReturnType::Type(_, t) => t,
        _ => return (false, output.clone()),
    };
    let bounds = match ty.as_ref() {
        syn::Type::ImplTrait(t) => &t.bounds,
        _ => return (false, output.clone()),
    };

    let output_ty = bounds
        .iter()
        .filter_map(|b| match b {
            syn::TypeParamBound::Trait(t) => Some(t),
            _ => None,
        })
        .find_map(|t| t.path.segments.last().filter(|s| s.ident == "Future"))
        .and_then(|s| match &s.arguments {
            syn::PathArguments::AngleBracketed(a) => Some(a),
            _ => None,
        })
        .and_then(|a| {
            a.args.iter().find_map(|arg| match arg {
                syn::GenericArgument::AssocType(at) if at.ident == "Output" => Some(at.ty.clone()),
                _ => None,
            })
        });

    output_ty
        .map(|t| (true, syn::ReturnType::Type(Default::default(), Box::new(t))))
        .unwrap_or((false, output.clone()))
}

/// Recursively replaces `Self` with `ident` in type paths, including inside generic args.
fn replace_self(ty: &mut syn::Type, ident: &syn::Ident) {
    let syn::Type::Path(p) = ty else { return };
    if p.path.segments.len() == 1 && p.path.segments[0].ident == "Self" {
        *ty = syn::Type::Path(syn::TypePath {
            qself: None,
            path: ident.clone().into(),
        });
        return;
    }
    p.path
        .segments
        .iter_mut()
        .filter_map(|s| match &mut s.arguments {
            syn::PathArguments::AngleBracketed(a) => Some(a),
            _ => None,
        })
        .flat_map(|a| a.args.iter_mut())
        .for_each(|arg| match arg {
            syn::GenericArgument::Type(t) => replace_self(t, ident),
            syn::GenericArgument::AssocType(at) => replace_self(&mut at.ty, ident),
            _ => {}
        });
}
