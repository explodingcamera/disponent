use proc_macro2::TokenStream;
use quote::quote;
use syn::{Result, spanned::Spanned};

pub fn impl_from(enum_def: &syn::ItemEnum) -> Result<TokenStream> {
    let variants = extract_variants(enum_def)?;
    let (impl_generics, ty_generics, where_clause) = enum_def.generics.split_for_impl();
    let enum_ident = &enum_def.ident;

    let impls = variants.iter().map(|(variant_ident, inner_ty, attrs)| {
        let attrs = attrs.iter().filter(|a| is_cfg_attr(a));
        quote! {
            #(#attrs)*
            #[automatically_derived]
            impl #impl_generics ::core::convert::From<#inner_ty> for #enum_ident #ty_generics #where_clause {
                fn from(val: #inner_ty) -> Self {
                    #enum_ident::#variant_ident(val)
                }
            }
        }
    });

    Ok(quote! { #(#impls)* })
}

pub fn impl_try_into(enum_def: &syn::ItemEnum) -> Result<TokenStream> {
    let variants = extract_variants(enum_def)?;
    let (impl_generics, ty_generics, where_clause) = enum_def.generics.split_for_impl();
    let enum_ident = &enum_def.ident;
    let enum_vis = &enum_def.vis;
    let error_ident = quote::format_ident!("{}TryIntoError", enum_ident);

    let impls = variants.iter().filter_map(|(variant_ident, inner_ty, attrs)| {
        if is_naked_type_param(inner_ty, enum_def) {
            return None;
        }
        let attrs = attrs.iter().filter(|a| is_cfg_attr(a));
        Some(quote! {
            #(#attrs)*
            #[automatically_derived]
            impl #impl_generics ::core::convert::TryFrom<#enum_ident #ty_generics> for #inner_ty #where_clause {
                type Error = #error_ident #ty_generics;

                fn try_from(value: #enum_ident #ty_generics) -> ::core::result::Result<#inner_ty, Self::Error> {
                    match value {
                        #enum_ident::#variant_ident(val) => Ok(val),
                        other => Err(#error_ident(other)),
                    }
                }
            }
        })
    });

    Ok(quote! {
        #enum_vis struct #error_ident #impl_generics (#enum_vis #enum_ident #ty_generics) #where_clause;

        impl #impl_generics ::core::fmt::Debug for #error_ident #ty_generics #where_clause {
            fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                f.debug_struct(stringify!(#error_ident)).finish_non_exhaustive()
            }
        }

        impl #impl_generics ::core::fmt::Display for #error_ident #ty_generics #where_clause {
            fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                write!(f, "try_into failed")
            }
        }

        impl #impl_generics ::core::error::Error for #error_ident #ty_generics #where_clause {}

        #(#impls)*
    })
}

fn extract_variants(
    enum_def: &syn::ItemEnum,
) -> Result<Vec<(&syn::Ident, &syn::Type, &Vec<syn::Attribute>)>> {
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
            "All variants must be newtype with one field for From/TryInto impls",
        ));
    }

    Ok(variants)
}

fn is_cfg_attr(attr: &syn::Attribute) -> bool {
    attr.path()
        .segments
        .last()
        .is_some_and(|s| s.ident == "cfg" || s.ident == "cfg_attr")
}

fn is_naked_type_param(ty: &syn::Type, enum_def: &syn::ItemEnum) -> bool {
    let syn::Type::Path(type_path) = ty else {
        return false;
    };
    if type_path.qself.is_some() || type_path.path.segments.len() != 1 {
        return false;
    }
    let segment = &type_path.path.segments[0];
    matches!(segment.arguments, syn::PathArguments::None)
        && enum_def
            .generics
            .type_params()
            .any(|param| param.ident == segment.ident)
}
