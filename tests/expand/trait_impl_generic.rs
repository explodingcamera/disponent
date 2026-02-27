disponent::declare!(
    #[disponent::configure(inline)]
    pub enum FooOrBar {
        Foo(Foo),
        Bar(Bar),
    }

    pub trait Convert<T> {
        fn convert(&self) -> T;
    }
);

pub struct Foo;
pub struct Bar;

impl Convert<i32> for Foo {
    fn convert(&self) -> i32 {
        42
    }
}

impl Convert<i32> for Bar {
    fn convert(&self) -> i32 {
        100
    }
}

fn main() {
    use Convert;
    let foo = FooOrBar::Foo(Foo);
    let val: i32 = foo.convert();
    assert_eq!(val, 42);
}
