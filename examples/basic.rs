disponent::declare!(
    #[disponent::configure(inherent, inline)]
    #[derive(Debug, Clone)]
    pub enum FooOrBar {
        Foo(Foo),
        Bar(Bar),
        #[cfg(test)]
        Buz(Buz),
    }

    pub trait SayHello {
        fn say_hello(&self) -> impl Future<Output = ()>;
        fn say_hello_send(&self) -> impl Future<Output = ()> + Send {
            async { println!("Default say_hello_send") }
        }
        fn name(&self) -> &'static str;
        fn with_lifetime<'a>(&self, s: &'a str) -> &'a str {
            s
        }
        fn with_generic<T: std::fmt::Display>(&self, val: T) -> String;
        fn consume(self) -> String;
        fn with_default(&self) -> &'static str {
            "default implementation"
        }
        #[cfg(test)]
        fn test_only(&self) -> bool;
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
    #[cfg(test)]
    fn test_only(&self) -> bool {
        true
    }
}

#[derive(Debug, Clone)]
pub struct Bar;
impl SayHello for Bar {
    fn say_hello(&self) -> impl Future<Output = ()> {
        async { println!("Hello from Bar") }
    }
    fn say_hello_send(&self) -> impl Future<Output = ()> + Send {
        async { println!("Hello send from Bar") }
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
    #[cfg(test)]
    fn test_only(&self) -> bool {
        true
    }
}

#[cfg(test)]
#[derive(Debug, Clone)]
pub struct Buz;

#[cfg(test)]
impl SayHello for Buz {
    async fn say_hello(&self) {
        println!("Hello from Buz")
    }
    fn say_hello_send(&self) -> impl Future<Output = ()> + Send {
        async { println!("Hello send from Buz") }
    }
    fn name(&self) -> &'static str {
        "Buz"
    }
    fn with_lifetime<'a>(&self, s: &'a str) -> &'a str {
        s
    }
    fn with_generic<T: std::fmt::Display>(&self, val: T) -> String {
        format!("Buz: {}", val)
    }
    fn consume(self) -> String {
        "consumed Buz".to_string()
    }
    fn test_only(&self) -> bool {
        true
    }
}

fn main() {
    smol::block_on(async {
        let foo_or_bar = FooOrBar::Foo(Foo);
        foo_or_bar.say_hello().await;
        foo_or_bar.say_hello_send().await;
        println!("name: {}", foo_or_bar.name());
        println!("with_lifetime: {}", foo_or_bar.with_lifetime("test"));
        println!("with_generic: {}", foo_or_bar.with_generic(42));
        println!("with_default: {}", foo_or_bar.with_default());

        let foo_or_bar = FooOrBar::Bar(Bar);
        foo_or_bar.say_hello().await;
        println!("name: {}", foo_or_bar.name());
        println!("with_generic: {}", foo_or_bar.with_generic(3.14));
        println!("with_default: {}", foo_or_bar.with_default());

        let consumed = foo_or_bar.consume();
        println!("consumed: {}", consumed);
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_buz_variant() {
        let buz = FooOrBar::Buz(Buz);
        assert_eq!(buz.name(), "Buz");
        assert!(buz.test_only());
    }
}
