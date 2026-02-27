use disponent::{configure, declare};

pub struct Foo;
pub struct Bar;

impl SayHello for Foo {
    fn say_hello(&self) {}
}
impl SayHello for Bar {
    fn say_hello(&self) {}
}

declare!(
    #[configure(inherent, from, try_into)]
    pub enum FooOrBar {
        Foo(Foo),
        Bar(Bar),
    }

    pub trait SayHello {
        fn say_hello(&self);
    }
);

fn main() {
    let foo: FooOrBar = Foo.into();
    assert!(matches!(foo, FooOrBar::Foo(_)));

    let bar: FooOrBar = Bar.into();
    assert!(matches!(bar, FooOrBar::Bar(_)));

    let foo: FooOrBar = Foo.into();
    let inner: Result<Foo, _> = foo.try_into();
    assert!(inner.is_ok());

    let bar: FooOrBar = Bar.into();
    let inner: Result<Foo, _> = bar.try_into();
    assert!(inner.is_err());
}
