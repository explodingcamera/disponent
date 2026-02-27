disponent::declare!(
    #[disponent::configure(inherent)]
    pub enum FooOrBar {
        Foo(Foo),
        Bar(Bar),
    }

    pub trait WithInner {
        fn method(&self, inner: i32) -> i32;
        fn another(&self, __disponent_inner: i32) -> i32;
    }
);

pub struct Foo;
impl WithInner for Foo {
    fn method(&self, inner: i32) -> i32 {
        inner
    }
    fn another(&self, __disponent_inner: i32) -> i32 {
        __disponent_inner
    }
}

pub struct Bar;
impl WithInner for Bar {
    fn method(&self, inner: i32) -> i32 {
        inner
    }
    fn another(&self, __disponent_inner: i32) -> i32 {
        __disponent_inner
    }
}

fn main() {}
