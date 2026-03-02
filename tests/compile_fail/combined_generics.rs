disponent::declare!(
    pub enum FooOrBar<T> {
        Foo(Foo<T>),
    }

    pub trait SayHello<U> {
        fn say_hello(&self, val: U);
    }
);

pub struct Foo<T>(std::marker::PhantomData<T>);

fn main() {}
