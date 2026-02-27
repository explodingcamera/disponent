use proc_macro2::TokenStream;
use quote::quote;
use syn::{Result, spanned::Spanned};

pub fn impl_from(enum_def: &syn::ItemEnum) -> Result<TokenStream> {
    let variants = extract_variants(enum_def)?;
    let (_, ty_generics, where_clause) = enum_def.generics.split_for_impl();
    let enum_ident = &enum_def.ident;

    let impls = variants.iter().map(|(variant_ident, inner_ty, attrs)| {
        let attrs = attrs.iter().filter(|a| is_cfg_attr(a));
        quote! {
            #(#attrs)*
            #[automatically_derived]
            impl ::core::convert::From<#inner_ty> for #enum_ident #ty_generics #where_clause {
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

    let impls = variants.iter().map(|(variant_ident, inner_ty, attrs)| {
        let attrs = attrs.iter().filter(|a| is_cfg_attr(a));
        quote! {
            #(#attrs)*
            #[automatically_derived]
            impl #impl_generics ::core::convert::TryInto<#inner_ty> for #enum_ident #ty_generics #where_clause {
                type Error = #error_ident<#enum_ident #ty_generics>;

                fn try_into(self) -> ::core::result::Result<#inner_ty, Self::Error> {
                    match self {
                        #enum_ident::#variant_ident(val) => Ok(val),
                        other => Err(#error_ident(other)),
                    }
                }
            }
        }
    });

    Ok(quote! {
        #enum_vis struct #error_ident<#enum_ident #ty_generics>(#enum_ident #ty_generics);

        impl<#impl_generics> ::core::fmt::Debug for #error_ident<#enum_ident #ty_generics> #where_clause {
            fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                f.debug_struct(stringify!(#error_ident)).finish_non_exhaustive()
            }
        }

        impl<#impl_generics> ::core::fmt::Display for #error_ident<#enum_ident #ty_generics> #where_clause {
            fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                write!(f, "try_into failed")
            }
        }

        impl<#impl_generics> ::core::error::Error for #error_ident<#enum_ident #ty_generics> #where_clause {}

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
