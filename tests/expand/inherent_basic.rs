use std::future::Future;

disponent::declare!(
    #[disponent::configure(inherent)]
    pub enum FooOrBar {
        Foo(Foo),
        Bar(Bar),
    }

    pub trait SayHello {
        fn say_hello(&self) -> impl Future<Output = ()>;
        fn name(&self) -> &'static str;
    }
);

pub struct Foo;
impl SayHello for Foo {
    async fn say_hello(&self) {}
    fn name(&self) -> &'static str {
        "Foo"
    }
}

pub struct Bar;
impl SayHello for Bar {
    fn say_hello(&self) -> impl Future<Output = ()> {
        async {}
    }
    fn name(&self) -> &'static str {
        "Bar"
    }
}

fn main() {}
