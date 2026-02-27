disponent::declare!(
    pub enum FooOrBar {
        Foo(Foo),
        Bar(Bar),
    }

    pub trait SayHello {
        fn name(&self) -> &'static str;
    }
);

pub struct Foo;
impl SayHello for Foo {
    fn name(&self) -> &'static str {
        "Foo"
    }
}

pub struct Bar;
impl SayHello for Bar {
    fn name(&self) -> &'static str {
        "Bar"
    }
}

fn main() {
    use SayHello;
    let foo = FooOrBar::Foo(Foo);
    assert_eq!(foo.name(), "Foo");
}
