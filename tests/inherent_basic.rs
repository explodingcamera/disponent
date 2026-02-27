use std::future::Future;

disponent::declare!(
    #[disponent::configure(inherent, inline)]
    pub enum FooOrBar {
        Foo(Foo),
        Bar(Bar),
    }

    pub trait SayHello {
        fn say_hello(&self) -> impl Future<Output = ()>;
        fn name(&self) -> &'static str;
        fn with_lifetime<'a>(&self, s: &'a str) -> &'a str {
            s
        }
        fn with_generic<T: std::fmt::Display>(&self, val: T) -> String;
        fn consume(self) -> String;
        fn with_default(&self) -> &'static str {
            "default implementation"
        }
    }
);

#[derive(Debug, Clone)]
pub struct Foo;

impl SayHello for Foo {
    async fn say_hello(&self) {
        println!("Hello from Foo")
    }
    fn name(&self) -> &'static str {
        "Foo"
    }
    fn with_generic<T: std::fmt::Display>(&self, val: T) -> String {
        format!("Foo: {}", val)
    }
    fn consume(self) -> String {
        "consumed Foo".to_string()
    }
}

#[derive(Debug, Clone)]
pub struct Bar;

impl SayHello for Bar {
    fn say_hello(&self) -> impl Future<Output = ()> {
        async { println!("Hello from Bar") }
    }
    fn name(&self) -> &'static str {
        "Bar"
    }
    fn with_generic<T: std::fmt::Display>(&self, val: T) -> String {
        format!("Bar: {}", val)
    }
    fn consume(self) -> String {
        "consumed Bar".to_string()
    }
}

#[test]
fn test_impl_future_async() {
    let foo = FooOrBar::Foo(Foo);
    let bar = FooOrBar::Bar(Bar);

    // Test impl Future<Output = ()> style
    smol::block_on(async {
        foo.say_hello().await;
        bar.say_hello().await;
    });
}

#[test]
fn test_name() {
    let foo = FooOrBar::Foo(Foo);
    let bar = FooOrBar::Bar(Bar);

    assert_eq!(foo.name(), "Foo");
    assert_eq!(bar.name(), "Bar");
}

#[test]
fn test_with_lifetime() {
    let foo = FooOrBar::Foo(Foo);
    let result = foo.with_lifetime("test");
    assert_eq!(result, "test");
}

#[test]
fn test_with_generic() {
    let foo = FooOrBar::Foo(Foo);
    let bar = FooOrBar::Bar(Bar);

    assert_eq!(foo.with_generic(42), "Foo: 42");
    assert_eq!(bar.with_generic(3.14), "Bar: 3.14");
}

#[test]
fn test_with_default() {
    let foo = FooOrBar::Foo(Foo);
    assert_eq!(foo.with_default(), "default implementation");
}

#[test]
fn test_consume() {
    let foo = FooOrBar::Foo(Foo);
    assert_eq!(foo.consume(), "consumed Foo");

    let bar = FooOrBar::Bar(Bar);
    assert_eq!(bar.consume(), "consumed Bar");
}
