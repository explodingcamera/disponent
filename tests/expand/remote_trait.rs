mod remote {
    pub trait SayHello {
        fn say_hello(&self);
    }
}

pub struct Foo;
pub struct Bar;

impl remote::SayHello for Foo {
    fn say_hello(&self) {}
}
impl remote::SayHello for Bar {
    fn say_hello(&self) {}
}

disponent::declare!(
    #[disponent::configure(inherent)]
    pub enum FooOrBar {
        Foo(Foo),
        Bar(Bar),
    }

    #[disponent::remote(remote::SayHello)]
    trait SayHello {
        fn say_hello(&self);
    }
);

fn main() {
    let foo = FooOrBar::Foo(Foo);
    foo.say_hello();

    let bar = FooOrBar::Bar(Bar);
    bar.say_hello();
}
