disponent::declare!(
    pub enum FooOrBar {
        Foo(Foo),
        Bar(Bar),
    }

    pub trait SayHello {
        #[allow(async_fn_in_trait)]
        async fn say_hello(&self);
    }
);

pub struct Foo;
impl SayHello for Foo {
    async fn say_hello(&self) {}
}

pub struct Bar;
impl SayHello for Bar {
    async fn say_hello(&self) {}
}

fn main() {
    use SayHello;
    smol::block_on(async {
        let foo = FooOrBar::Foo(Foo);
        foo.say_hello().await;
    });
}
