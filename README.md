# `disponent`

[<img alt="github" src="https://img.shields.io/badge/github-explodingcamera/disponent-8da0cb?style=flat-square&labelColor=555555&logo=github" height="20">](https://github.com/explodingcamera/disponent)
[<img alt="crates.io" src="https://img.shields.io/crates/v/disponent.svg?style=flat-square&color=fc8d62&logo=rust" height="20">](https://crates.io/crates/disponent)
[<img alt="docs.rs" src="https://img.shields.io/badge/docs.rs-disponent-66c2a5?style=flat-square&labelColor=555555&logo=docs.rs" height="20">](https://docs.rs/disponent)
[<img alt="build status" src="https://img.shields.io/github/actions/workflow/status/explodingcamera/disponent/ci.yml?branch=main&style=flat-square" height="20">](https://github.com/explodingcamera/disponent/actions?query=branch%3Amain)

`disponent` is an alternative to using `dyn Trait` trait objects for dispatching to multiple implementations of a trait. Works with async methods, generics, `#[cfg]` attributes, `no_std` and even traits that are not object safe.

## Usage

```rust
use disponent::declare;

declare!(
    pub enum FooOrBar {
        Foo(Foo),
        Bar(Bar),
    }

    pub trait SayHello {
        fn say_hello(&self) -> impl Future<Output = ()>;
        fn name(&self) -> &'static str;

        // Default implementations work too
        fn with_default(&self) -> &'static str {
            "default"
        }
    }
);

pub struct Foo;
impl SayHello for Foo {
    async fn say_hello(&self) {
        println!("Hello from Foo")
    }
    fn name(&self) -> &'static str { "Foo" }
}

pub struct Bar;
impl SayHello for Bar {
    async fn say_hello(&self) {
        println!("Hello from Bar")
    }
    fn name(&self) -> &'static str { "Bar" }
}

fn main() {
    // `FooOrBar` implements `SayHello` by delegating to the inner type
    let foo_or_bar = FooOrBar::Foo(Foo);
    smol::block_on(foo_or_bar.say_hello());
    println!("My name is {}", foo_or_bar.name());
}
```

## Configuration Options

Apply `#[disponent::configure(...)]` to the enum with any combination of:

- `inherent`: Generate inherent methods on the enum (vs trait impl)
- `inline`: Add `#[inline]` to all generated methods
- `from`: Generate `From` impls for each variant
- `try_into`: Generate `TryInto` impls for each variant

## Generated Code

The above example generates the following code:

```rust
#[automatically_derived]
impl SayHello for FooOrBar {
    async fn say_hello(&self) -> () {
        match self {
            FooOrBar::Foo(inner) => SayHello::say_hello(inner).await,
            FooOrBar::Bar(inner) => SayHello::say_hello(inner).await,
        }
    }
    fn name(&self) -> &'static str {
        match self {
            FooOrBar::Foo(inner) => SayHello::name(inner),
            FooOrBar::Bar(inner) => SayHello::name(inner),
        }
    }
    fn with_default(&self) -> &'static str {
        match self {
            FooOrBar::Foo(inner) => SayHello::with_default(inner),
            FooOrBar::Bar(inner) => SayHello::with_default(inner),
        }
    }
}
```

In many cases, this can be substantially faster than using `dyn Trait` trait objects, especially when the enum is small and the methods are simple. See [the benchmarks of `enum_dispatch`](https://docs.rs/enum_dispatch/latest/enum_dispatch/#performance) for more details (benchmarks for `disponent` are coming soon).

## See also

- [**`declarative_enum_dispatch`**](https://crates.io/crates/declarative_enum_dispatch)
- [**`enum_delegate`**](https://crates.io/crates/enum_delegate)
- [**`enum_dispatch`**](https://crates.io/crates/enum_dispatch)
- [**`enum_derive`**](https://crates.io/crates/enum_derive)
- [**`ambassador`**](https://crates.io/crates/ambassador)
- [**`delegation`**](https://crates.io/crates/delegation)

With the exception of `declarative_enum_dispatch`, these crates all share state between macro invocations, which can lead to issues with `rust-analyzer` and similar tools. `disponent` avoids this by generating all code in a single macro invocation, at the cost of some flexibility in how the traits are defined (all while `rust-fmt` and error reporting works like normal).

## License

Licensed under either of [Apache License, Version 2.0](./LICENSE-APACHE) or [MIT license](./LICENSE-MIT) at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in disponent by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.
