disponent::declare!(
    #[disponent::configure(inherent)]
    pub enum FooOrBar {
        Foo(Foo),
        Bar(Bar),
    }

    pub trait Bad {
        fn method(self: Box<Self>);
    }
);

pub struct Foo;
pub struct Bar;

fn main() {}
