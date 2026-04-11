disponent::declare!(
    #[disponent::configure(inherent)]
    pub enum Factory {
        A(A),
        B(B),
    }

    pub trait Build {
        fn make() -> Self;
    }
);

pub struct A;
pub struct B;

impl Build for A {
    fn make() -> Self {
        A
    }
}

impl Build for B {
    fn make() -> Self {
        B
    }
}

fn main() {}
