disponent::declare!(
    #[disponent::configure(inherent)]
    pub enum FooOrBar {
        Foo(Foo),
        Bar(Bar),
    }

    pub trait Convert<T> {
        fn convert(&self) -> T;
        fn convert_with<T: Clone>(&self, extra: T) -> String;
    }
);

pub struct Foo;
pub struct Bar;

fn main() {}
