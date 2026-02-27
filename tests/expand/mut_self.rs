disponent::declare!(
    #[disponent::configure(inherent)]
    pub enum FooOrBar {
        Foo(Foo),
        Bar(Bar),
    }

    pub trait Mutate {
        fn mutate(&mut self, val: i32);
    }
);

pub struct Foo(i32);
impl Mutate for Foo {
    fn mutate(&mut self, val: i32) {
        self.0 = val;
    }
}

pub struct Bar(i32);
impl Mutate for Bar {
    fn mutate(&mut self, val: i32) {
        self.0 = val;
    }
}

fn main() {
    let mut foo = FooOrBar::Foo(Foo(0));
    foo.mutate(42);
}
