disponent::declare!(
    #[disponent::configure(inherent)]
    pub enum FooOrBar {
        Foo(Foo),
        Bar(Bar),
    }

    pub trait Convert<T> {
        fn convert(&self) -> T;
    }
);

pub struct Foo;
impl Convert<i32> for Foo {
    fn convert(&self) -> i32 {
        42
    }
}

pub struct Bar;
impl Convert<i32> for Bar {
    fn convert(&self) -> i32 {
        100
    }
}

fn main() {
    let foo = FooOrBar::Foo(Foo);
    let val: i32 = foo.convert();
    assert_eq!(val, 42);
}
