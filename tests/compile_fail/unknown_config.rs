disponent::declare!(
    #[disponent::configure(unknown_option)]
    pub enum FooOrBar {
        Foo(Foo),
    }

    pub trait SayHello {
        fn say_hello(&self);
    }
);

pub struct Foo;

fn main() {}
