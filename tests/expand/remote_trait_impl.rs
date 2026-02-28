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
    remote::SayHello::say_hello(&foo);

    let bar = FooOrBar::Bar(Bar);
    remote::SayHello::say_hello(&bar);

    fn takes_say_hello(s: &impl remote::SayHello) {
        s.say_hello();
    }
    takes_say_hello(&foo);
    takes_say_hello(&bar);
}
