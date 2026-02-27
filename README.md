# `disponent` - ergonomic enum delegation

`disponent` is an alternative to using `dyn Trait` trait objects for dispatching to multiple implementations of a trait, without the need for object safety.

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

## See also

- [**`declarative_enum_dispatch`**](https://crates.io/crates/declarative_enum_dispatch)
- [**`enum_delegate`**](https://crates.io/crates/enum_delegate)
- [**`enum_dispatch`**](https://crates.io/crates/enum_dispatch)
- [**`enum_derive`**](https://crates.io/crates/enum_derive)
- [**`ambassador`**](https://crates.io/crates/ambassador)
- [**`delegation`**](https://crates.io/crates/delegation)

With the exception of `declarative_enum_dispatch`, these crates all share state between macro invocations, which can lead to issues with `rust-analyzer` and similar tools. `disponent` avoids this by generating all code in a single macro invocation, at the cost of some flexibility in how the traits are defined (all while `rust-fmt` and error reporting works like normal).
