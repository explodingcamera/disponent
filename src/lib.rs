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
//! - `from`: Generate `From` impls for each variant via `From<VariantInner> for Enum`
//! - `try_into`: Generate conversion support via `TryFrom<Enum> for VariantInner`
//!
//! ## Remote Traits
//!
//! Use [`#[disponent::remote(...)]`][remote] on the trait to implement a trait defined elsewhere.
//!
//! ## Fallback Variant for Methods Without Receiver
//!
//! For trait methods without a receiver (for example `fn make() -> Self`), mark exactly one enum
//! variant with `#[fallback]`.
//!
//! `disponent` forwards no-receiver methods to the fallback variant's inner type. For `-> Self`,
//! the inner return value is wrapped into the fallback enum variant.

mod convert;
mod forward;

use proc_macro2::{Delimiter, Group, TokenStream, TokenTree};
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

        let file = match syn::parse2::<syn::File>(input) {
            Ok(f) => f,
            Err(_) => return Ok(Disponent(out)),
        };
        let items = &file.items;

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

        for attr in &enum_def.attrs {
            let is_configure = attr
                .path()
                .segments
                .last()
                .is_some_and(|segment| segment.ident == "configure");
            let path = attr.path();
            let is_allowed_configure_path = path.is_ident("configure")
                || (path.segments.len() == 2
                    && path.segments[0].ident == "disponent"
                    && path.segments[1].ident == "configure");

            if is_configure && !is_allowed_configure_path {
                return Err(syn::Error::new(
                    path.span(),
                    "Inside declare!, use #[configure(...)] or #[disponent::configure(...)] (do not rename configure)",
                ));
            }

            if !is_allowed_configure_path && attr.parse_args::<Configure>().is_ok() {
                return Err(syn::Error::new(
                    path.span(),
                    "Inside declare!, use #[configure(...)] or #[disponent::configure(...)] (do not rename configure)",
                ));
            }
        }

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

        let has_fallback_attr = enum_def.variants.iter().any(|variant| {
            variant
                .attrs
                .iter()
                .any(|attr| attr.path().is_ident("fallback"))
        });

        let declaration_input = if has_fallback_attr {
            strip_fallback_attrs(out.clone())
        } else {
            out
        };

        let definition = quote::quote! {
            #declaration_input
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

fn strip_fallback_attrs(tokens: TokenStream) -> TokenStream {
    strip_fallback_attrs_inner(tokens).0
}

fn strip_fallback_attrs_inner(tokens: TokenStream) -> (TokenStream, bool) {
    let mut out = TokenStream::new();
    let mut iter = tokens.into_iter().peekable();
    let mut changed = false;

    while let Some(token) = iter.next() {
        match token {
            TokenTree::Punct(punct) if punct.as_char() == '#' => {
                let should_strip = iter.peek().and_then(|next| match next {
                    TokenTree::Group(group) if group.delimiter() == Delimiter::Bracket => {
                        Some(is_fallback_attr_group(group))
                    }
                    _ => None,
                });

                if should_strip == Some(true) {
                    iter.next();
                    changed = true;
                    continue;
                }

                out.extend(std::iter::once(TokenTree::Punct(punct)));
            }
            TokenTree::Group(group) => {
                let (inner, inner_changed) = strip_fallback_attrs_inner(group.stream());
                if inner_changed {
                    changed = true;
                    let mut rewritten = Group::new(group.delimiter(), inner);
                    rewritten.set_span(group.span());
                    out.extend(std::iter::once(TokenTree::Group(rewritten)));
                } else {
                    out.extend(std::iter::once(TokenTree::Group(group)));
                }
            }
            other => out.extend(std::iter::once(other)),
        }
    }

    (out, changed)
}

fn is_fallback_attr_group(group: &Group) -> bool {
    syn::parse2::<syn::Path>(group.stream())
        .map(|path| path.is_ident("fallback"))
        .unwrap_or(false)
}

/// Declare a trait and enum together, generating forwarding methods.
///
/// Enum variants must be newtype fields (single unnamed field). Each variant's inner type
/// must implement the declared trait.
///
/// Methods without a receiver require exactly one enum variant marked `#[fallback]`.
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
/// - `try_into`: Generate `TryFrom<Enum> for VariantInner` impls (enables `.try_into()` via blanket impl)
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
