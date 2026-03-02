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

enum InherentConfig {
    Inherit,
    Explicit(syn::Visibility),
}

#[derive(Default)]
struct Configure {
    inherent: Option<InherentConfig>,
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
        let mut inherent: Option<InherentConfig> = None;
        let mut from = false;
        let mut try_into = false;
        let mut inline = false;

        while !input.is_empty() {
            let ident: syn::Ident = input.parse()?;
            match ident.to_string().as_str() {
                "inherent" => {
                    if input.peek(syn::token::Paren) {
                        let content;
                        syn::parenthesized!(content in input);
                        let vis: syn::Visibility = content.parse()?;
                        inherent = Some(InherentConfig::Explicit(vis));
                    } else {
                        inherent = Some(InherentConfig::Inherit);
                    }
                }
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
        let input: TokenStream = input.parse()?;
        let out = input.clone();

        let items = match syn::parse2::<syn::File>(input) {
            Ok(f) => f.items,
            Err(_) => return Ok(Disponent(out)),
        };

        let trait_def = items
            .iter()
            .find_map(|item| match item {
                syn::Item::Trait(t) => Some(t.clone()),
                _ => None,
            })
            .ok_or_else(|| syn::Error::new(out.span(), "Missing trait definition"))?;

        let enum_def = items
            .iter()
            .find_map(|item| match item {
                syn::Item::Enum(e) => Some(e.clone()),
                _ => None,
            })
            .ok_or_else(|| syn::Error::new(out.span(), "Missing enum definition"))?;

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

        let remote_path = trait_def
            .attrs
            .iter()
            .find(|attr| {
                attr.path()
                    .segments
                    .last()
                    .is_some_and(|segment| segment.ident == "remote")
            })
            .map(|attr| attr.parse_args::<Remote>())
            .transpose()?
            .map(|remote| remote.path);

        let forward_to_variant = forward::forward_to_variant(
            config.inherent.as_ref(),
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

        let definition = quote::quote! {
            #out
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
/// - `inherent`: Generate inherent methods on the enum with the same visibility as the enum
/// - `inherent(<visibility>)`: Generate inherent methods with explicit visibility (e.g., `inherent(pub)`, `inherent(pub(crate))`)
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
    let mut item = match syn::parse(out.clone()) {
        Ok(syn::Item::Enum(enum_def)) => enum_def,
        _ => {
            return quote::quote! {
                compile_error!("The #[disponent::configure] attribute can only be applied to enums within the declare! macro");
            }.into();
        }
    };

    item.attrs.retain(|attr| {
        attr.path()
            .segments
            .last()
            .is_none_or(|segment| segment.ident != "configure")
    });

    quote::quote!(#item).into()
}

// TODO: support for multiple traits and/or traits in the same macro invocation

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
    let mut item = match syn::parse(out.clone()) {
        Ok(syn::Item::Trait(trait_def)) => trait_def,
        _ => {
            return quote::quote! {
                compile_error!("The #[disponent::remote] attribute can only be applied to traits within the declare! macro");
            }.into();
        }
    };

    item.attrs.push(syn::parse_quote!(#[doc(hidden)]));
    item.attrs.push(syn::parse_quote!(#[allow(unused)]));
    quote::quote!(#item).into()
}
