disponent::declare!(
    pub enum FooOrBar {
        Foo(Foo),
    }

    pub trait WithAssocType {
        type Output;
        fn get(&self) -> Self::Output;
    }
);

pub struct Foo;

fn main() {}
