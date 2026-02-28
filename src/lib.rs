//! # `disponent`
//!
//! An alternative to `dyn Trait` for dispatching to multiple implementations.
//!
//! ## Example
//!
//! Use the [`declare!`] macro to define a trait and enum together. The enum will
//! implement the trait by delegating method calls to its variants.
//!
//! ```rust
//! use disponent::declare;
//!
//! declare!(
//!     pub enum FooOrBar {
//!         Foo(Foo),
//!         Bar(Bar),
//!     }
//!
//!     pub trait SayHello {
//!         fn say_hello(&self);
//!     }
//! );
//! #
//! # struct Foo;
//! # struct Bar;
//! # impl SayHello for Foo { fn say_hello(&self) {} }
//! # impl SayHello for Bar { fn say_hello(&self) {} }
//! ```
//!
//! ## Configuration
//!
//! Use [`#[disponent::configure(...)]`][configure] on the enum with:
//! - `inherent`: Generate inherent methods (vs trait impl)
//! - `inline`: Add `#[inline]` to methods
//! - `from`: Generate `From<T> for Enum` impls
//! - `try_into`: Generate `TryInto<T> for Enum` impls
//!
//! ## Remote Traits
//!
//! Use [`#[disponent::remote(...)]`][remote] on the trait to implement a trait defined elsewhere.

mod convert;
mod forward;

use proc_macro2::TokenStream;
use quote::ToTokens;
use syn::{
    Result,
    parse::{Parse, ParseStream},
    parse_macro_input,
    spanned::Spanned,
};

#[derive(Default)]
struct Configure {
    inherent: bool,
    from: bool,
    try_into: bool,
    inline: bool,
}

struct Remote {
    path: syn::Path,
}

impl Parse for Remote {
    fn parse(input: ParseStream) -> Result<Self> {
        Ok(Remote {
            path: input.parse()?,
        })
    }
}

impl Parse for Configure {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut inherent = false;
        let mut from = false;
        let mut try_into = false;
        let mut inline = false;

        while !input.is_empty() {
            let ident: syn::Ident = input.parse()?;
            match ident.to_string().as_str() {
                "inherent" => inherent = true,
                "from" => from = true,
                "try_into" => try_into = true,
                "inline" => inline = true,
                _ => {
                    return Err(syn::Error::new(
                        ident.span(),
                        "Unknown configuration option",
                    ));
                }
            }
            if input.peek(syn::Token![,]) {
                input.parse::<syn::Token![,]>()?;
            }
        }

        Ok(Configure {
            inherent,
            from,
            try_into,
            inline,
        })
    }
}

struct Disponent(TokenStream);

impl Parse for Disponent {
    fn parse(input: ParseStream) -> Result<Self> {
        let input = input.parse::<TokenStream>()?;

        let mut items = match syn::parse2::<syn::File>(input.clone()) {
            Ok(f) => f.items,
            Err(_) => return Ok(Disponent(input)),
        };

        let trait_idx = items
            .iter()
            .position(|item| matches!(item, syn::Item::Trait(_)))
            .ok_or_else(|| syn::Error::new(input.span(), "Missing trait definition"))?;

        let enum_def = items
            .iter()
            .find_map(|item| match item {
                syn::Item::Enum(e) => Some(e.clone()),
                _ => None,
            })
            .ok_or_else(|| syn::Error::new(input.span(), "Missing enum definition"))?;

        let config = enum_def
            .attrs
            .iter()
            .find(|attr| {
                attr.path()
                    .segments
                    .last()
                    .is_some_and(|segment| segment.ident == "configure")
            })
            .map(|attr| attr.parse_args::<Configure>())
            .transpose()?
            .unwrap_or_default();

        let remote_path = if let syn::Item::Trait(trait_def) = &mut items[trait_idx] {
            let remote_attr_idx = trait_def.attrs.iter().position(|attr| {
                attr.path()
                    .segments
                    .last()
                    .is_some_and(|segment| segment.ident == "remote")
            });

            if let Some(idx) = remote_attr_idx {
                let remote_attr = trait_def.attrs.remove(idx);
                let remote = remote_attr.parse_args::<Remote>()?;
                trait_def.ident = quote::format_ident!("{}Remote", trait_def.ident);
                trait_def.attrs.push(syn::parse_quote!(#[allow(dead_code)]));
                trait_def.attrs.push(syn::parse_quote!(#[doc(hidden)]));
                Some(remote.path)
            } else {
                None
            }
        } else {
            None
        };

        let trait_def = match &items[trait_idx] {
            syn::Item::Trait(t) => t.clone(),
            _ => unreachable!(),
        };

        let forward_to_variant = forward::forward_to_variant(
            config.inherent,
            config.inline,
            &enum_def,
            &trait_def,
            remote_path.as_ref(),
        )?;
        let from_impl = if config.from {
            convert::impl_from(&enum_def)?
        } else {
            TokenStream::new()
        };
        let try_into_impl = if config.try_into {
            convert::impl_try_into(&enum_def)?
        } else {
            TokenStream::new()
        };

        let modified_input: TokenStream = items.iter().map(|i| i.to_token_stream()).collect();

        let definition = quote::quote! {
            #modified_input

            #forward_to_variant
            #from_impl
            #try_into_impl
        };

        Ok(Disponent(definition))
    }
}

impl ToTokens for Disponent {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.0.to_tokens(tokens)
    }
}

/// Declare a trait and enum together, generating forwarding methods.
///
/// Enum variants must be newtype fields (single unnamed field). Each variant's inner type
/// must implement the declared trait.
///
/// Use [`#[disponent::configure(...)]`][configure] on the enum for options like `inherent` or `from`.
/// Use [`#[disponent::remote(...)]`][remote] on the trait to implement a remote trait.
///
/// # Example
///
/// ```rust
/// use disponent::declare;
/// # struct Foo;
/// # struct Bar;
/// # impl SayHello for Foo { fn say_hello(&self) {} }
/// # impl SayHello for Bar { fn say_hello(&self) {} }
///
/// declare!(
///     pub enum FooOrBar {
///         Foo(Foo),
///         Bar(Bar),
///     }
///
///     pub trait SayHello {
///         fn say_hello(&self);
///     }
/// );
/// ```
#[proc_macro]
pub fn declare(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let disponent = parse_macro_input!(input as Disponent);
    disponent.into_token_stream().into()
}

/// Configure the enum forwarding behavior.
///
/// Apply to the enum within [`declare!`] with any combination of:
/// - `inherent`: Generate inherent methods on the enum (vs trait impl)
/// - `inline`: Add `#[inline]` to all generated methods
/// - `from`: Generate `From` impls for each variant
/// - `try_into`: Generate `TryInto` impls for each variant
///
/// # Example
///
/// ```rust
/// use disponent::declare;
/// # struct Foo;
/// # struct Bar;
/// # impl SayHello for Foo { fn say_hello(&self) {} }
/// # impl SayHello for Bar { fn say_hello(&self) {} }
///
/// declare!(
///     #[disponent::configure(inherent, inline, from, try_into)]
///     pub enum FooOrBar {
///         Foo(Foo),
///         Bar(Bar),
///     }
///
///     pub trait SayHello { fn say_hello(&self); }
/// );
/// ```
#[proc_macro_attribute]
pub fn configure(
    _input: proc_macro::TokenStream,
    out: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    // this is just used to make rust-analyzer happy, the actual parsing is done in the [`declare`] macro
    out
}

// TODO: support for multiple traits and/or traits in the same macro invocation
// TODO: add a inherent(visibility) option to inherent to override the visibility of the generated inherent impl
// e.g `inherent(pub)`, `inherent(pub(crate))`, `inherent(pub(super))`, `inherent` (same as enum)

/// Use a remote trait instead of the declared trait.
///
/// Apply to the trait within [`declare!`] with the path to a trait defined elsewhere.
/// The local trait is renamed and hidden; the remote trait is implemented.
///
/// # Example
///
/// ```rust
/// use disponent::declare;
/// # mod external { pub trait SayHello { fn say_hello(&self); } }
/// # struct Foo;
/// # struct Bar;
/// # impl external::SayHello for Foo { fn say_hello(&self) {} }
/// # impl external::SayHello for Bar { fn say_hello(&self) {} }
///
/// declare!(
///     pub enum FooOrBar {
///         Foo(Foo),
///         Bar(Bar),
///     }
///
///     #[disponent::remote(external::SayHello)]
///     trait SayHello { fn say_hello(&self); }
/// );
/// ```
#[proc_macro_attribute]
pub fn remote(
    _input: proc_macro::TokenStream,
    out: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    // this is just used to make rust-analyzer happy, the actual parsing is done in the [`declare`] macro
    out
}
