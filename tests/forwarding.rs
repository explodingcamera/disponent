disponent::declare! {
    enum Wrapper<T: Build> {
        #[fallback]
        Value(T),
    }

    trait Build: Sized {
        fn duplicate(self) -> Self;
        fn make() -> Self;
        fn size<U, const N: usize>(&self) -> usize;
        fn static_size<U, const N: usize>() -> usize;
        fn borrow<'a, U>(&self, value: &'a U) -> &'a U;
    }

    const FALLBACK_TOKENS: &str = stringify!(#[fallback]);
}

impl Build for u8 {
    fn duplicate(self) -> Self {
        self
    }
    fn make() -> Self {
        7
    }
    fn size<U, const N: usize>(&self) -> usize {
        size_of::<U>() + N
    }
    fn static_size<U, const N: usize>() -> usize {
        size_of::<U>() + N
    }
    fn borrow<'a, U>(&self, value: &'a U) -> &'a U {
        value
    }
}

disponent::declare! {
    #[disponent::configure(inherent)]
    enum Inherent<T: Build> {
        #[fallback]
        Value(T),
    }

    #[disponent::remote(Build)]
    trait BuildMirror {
        fn duplicate(self) -> Self;
        fn make() -> Self;
        fn size<U, const N: usize>(&self) -> usize;
        fn static_size<U, const N: usize>() -> usize;
    }
}

#[test]
fn generic_self_returns() {
    assert!(matches!(Wrapper::Value(5u8).duplicate(), Wrapper::Value(5)));
    assert!(matches!(Wrapper::<u8>::make(), Wrapper::Value(7)));
    assert!(matches!(
        Inherent::Value(5u8).duplicate(),
        Inherent::Value(5)
    ));
    assert!(matches!(Inherent::<u8>::make(), Inherent::Value(7)));
}

#[test]
fn forwards_method_generics() {
    assert_eq!(Wrapper::Value(0u8).size::<u16, 3>(), 5);
    assert_eq!(Wrapper::<u8>::static_size::<u32, 3>(), 7);
    assert_eq!(Inherent::Value(0u8).size::<u16, 3>(), 5);
    assert_eq!(Inherent::<u8>::static_size::<u32, 3>(), 7);
    let value = String::from("borrowed");
    assert!(std::ptr::eq(Wrapper::Value(0u8).borrow(&value), &value));
}

#[test]
fn preserves_macro_input() {
    assert_eq!(FALLBACK_TOKENS, stringify!(#[fallback]));
}
