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

        let items = match syn::parse2::<syn::File>(input.clone()) {
            Ok(f) => f.items,
            Err(_) => return Ok(Disponent(input)),
        };

        let trait_def = items
            .iter()
            .find_map(|item| match item {
                syn::Item::Trait(t) => Some(t.clone()),
                _ => None,
            })
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

        let forward_to_variant =
            forward::forward_to_variant(config.inherent, config.inline, &enum_def, &trait_def)?;
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
            #input

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

/// Declare a trait and enum together, generating forwarding methods that delegate from the enum to its variants.
///
/// The enum variants must all be newtype variants (single unnamed field). Each variant's inner type must implement the declared trait.
///
/// ## Example
/// ```rust
/// # use disponent::{configure, declare};
/// # struct Foo;
/// # struct Bar;
/// # impl SayHello for Foo {
/// #     fn say_hello(&self) -> impl Future<Output = ()> {
/// #         async { println!("Foo") }
/// #     }
/// # }
/// # impl SayHello for Bar {
/// #     async fn say_hello(&self) {
/// #         println!("Bar")
/// #     }
/// # }
/// declare!(
///     #[disponent::configure(inherent)]
///     pub enum FooOrBar {
///         Foo(Foo),
///         Bar(Bar),
///     }
///
///     pub trait SayHello {
///         fn say_hello(&self) -> impl Future<Output = ()>;
///     }
/// );
/// ```
#[proc_macro]
pub fn declare(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let disponent = parse_macro_input!(input as Disponent);
    disponent.into_token_stream().into()
}

/// Custom configuration for the `disponent` macro.
///
/// This is used to specify how the forwarding methods should be generated, and whether to generate [`From`]/[`TryInto`] impls for the enum variants.
///
/// ## Options
/// - `inherent`: If set, the forwarding methods will be inherent methods on the enum that forward to the variants using the trait. If not set, the forwarding methods will be trait impls for the enum where the methods forward to the variants using the trait.
/// - `inline`: If set, adds `#[inline]` to all generated methods.
/// - `from`: If set, impls `From` for each variant, so you can convert each variant into the enum.
/// - `try_into`: If set, impls `TryInto` for each variant, so you can try to convert the enum into a specific variant, and get an error if it's the wrong variant.
///
/// ## Example
/// ```rust
/// # struct Foo;
/// # struct Bar;
/// #[disponent::configure(inherent, inline, from, try_into)]
/// pub enum FooOrBar {
///   Foo(Foo),
///   Bar(Bar),
/// }
/// ```
#[proc_macro_attribute]
pub fn configure(
    _input: proc_macro::TokenStream,
    out: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    // this is just used to make rust-analyzer happy, the actual parsing is done in the `disponent` macro
    out
}

// TODO: support for multiple traits and/or traits in the same macro invocation
// TODO: add a inherent(visibility) option to inherent to override the visibility of the generated inherent impl
// e.g `inherent(pub)`, `inherent(pub(crate))`, `inherent(pub(super))`, `inherent` (same as enum)
// TODO: support for remote traits
// /// Specify a remote trait to use that matches the definition of the trait in the `disponent` macro.\
// /// The path should be the full path to the trait, including the crate name if it's from another crate.\
// ///
// /// This can be useful when using a trait from another crate or module.
// ///
// /// # Example
// /// ```rust
// /// #[disponent::remote(crate::SayHello)]
// /// trait SayHello {
// ///    fn say_hello(&self) -> impl Future<Output = ()>;
// /// }
// /// ```
// #[proc_macro_attribute]
// pub fn remote(
//     _input: proc_macro::TokenStream,
//     out: proc_macro::TokenStream,
// ) -> proc_macro::TokenStream {
//     // this is just used to make rust-analyzer happy, the actual parsing is done in the `disponent` macro
//     out
// }
