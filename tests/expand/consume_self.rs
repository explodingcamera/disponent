disponent::declare!(
    #[disponent::configure(inherent)]
    pub enum FooOrBar {
        Foo(Foo),
        Bar(Bar),
    }

    pub trait Consume {
        fn consume(self) -> String;
    }
);

pub struct Foo;
impl Consume for Foo {
    fn consume(self) -> String {
        "Foo".into()
    }
}

pub struct Bar;
impl Consume for Bar {
    fn consume(self) -> String {
        "Bar".into()
    }
}

fn main() {
    let foo = FooOrBar::Foo(Foo);
    let _ = foo.consume();
}
