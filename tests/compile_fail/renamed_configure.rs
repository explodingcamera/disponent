use disponent::configure as myconfigure;

disponent::declare!(
    #[myconfigure(inherent)]
    pub enum FooOrBar {
        Foo(Foo),
        Bar(Bar),
    }

    pub trait SayHello {
        fn say_hello(&self);
    }
);

pub struct Foo;
pub struct Bar;

impl SayHello for Foo {
    fn say_hello(&self) {}
}

impl SayHello for Bar {
    fn say_hello(&self) {}
}

fn main() {}
