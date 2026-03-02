disponent::declare!(
    pub enum FooOrBar {
        Foo(Foo),
    }

    pub trait WithAssocConst {
        const VALUE: usize;
        fn get(&self) -> usize;
    }
);

pub struct Foo;

fn main() {}
