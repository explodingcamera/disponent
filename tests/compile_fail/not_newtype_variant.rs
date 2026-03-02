disponent::declare!(
    pub enum FooOrBar {
        Foo(Foo, Bar),
        Baz(Baz),
    }

    pub trait SayHello {
        fn say_hello(&self);
    }
);

pub struct Foo;
pub struct Bar;
pub struct Baz;

fn main() {}
