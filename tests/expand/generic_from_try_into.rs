disponent::declare!(
    #[disponent::configure(from, try_into)]
    pub enum Wrapper<T> {
        Value(Value<T>),
    }

    pub trait Marker {
        fn is_valid(&self) -> bool;
    }
);

impl Marker for i32 {
    fn is_valid(&self) -> bool {
        true
    }
}

pub struct Value<T>(pub T);

impl<T> Marker for Value<T> {
    fn is_valid(&self) -> bool {
        true
    }
}

fn main() {
    let wrapped: Wrapper<i32> = Value(5_i32).into();
    let value: Result<Value<i32>, _> = wrapped.try_into();
    assert_eq!(value.unwrap().0, 5);
}
