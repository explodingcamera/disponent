disponent::declare!(
    #[disponent::configure(from, try_into)]
    pub enum Wrapper<T> {
        Value(T),
    }

    pub trait Marker {}
);

fn main() {
    let _wrapped: Wrapper<i32> = 5_i32.into();
}
