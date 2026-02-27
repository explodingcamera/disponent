disponent::declare!(
    #[disponent::configure(inherent)]
    pub enum FooOrBar {
        Foo(Foo),
        Bar(Bar),
    }

    pub trait Convert<T> {
        fn convert(&self) -> T;
        fn convert_with<U: Clone + ToString>(&self, extra: U) -> String;
    }
);

pub struct Foo;
impl Convert<i32> for Foo {
    fn convert(&self) -> i32 {
        42
    }
    fn convert_with<U: Clone + ToString>(&self, extra: U) -> String {
        format!("Foo: {}", extra.to_string())
    }
}

pub struct Bar;
impl Convert<i32> for Bar {
    fn convert(&self) -> i32 {
        100
    }
    fn convert_with<U: Clone + ToString>(&self, extra: U) -> String {
        format!("Bar: {}", extra.to_string())
    }
}

fn main() {
    let foo = FooOrBar::Foo(Foo);
    let result = foo.convert_with("hello");
    assert_eq!(result, "Foo: hello");
}
