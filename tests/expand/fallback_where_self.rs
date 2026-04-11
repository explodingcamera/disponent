disponent::declare!(
    #[disponent::configure(inherent)]
    pub enum Factory {
        #[fallback]
        A(A),
        B(B),
    }

    pub trait Build {
        fn using_fallback() -> Self
        where
            Self: Default,
        {
            Self::default()
        }
    }
);

#[derive(Default)]
pub struct A;
pub struct B;

impl Build for A {}
impl Build for B {}

fn main() {
    let built = Factory::using_fallback();
    assert!(matches!(built, Factory::A(_)));
}
