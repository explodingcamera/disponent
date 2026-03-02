disponent::declare!(
    #[disponent::configure(inherent(pub(crate)))]
    pub enum FooOrBar {
        Foo(Foo),
        Bar(Bar),
    }

    pub trait SayHello {
        fn say_hello(&self);
    }
);

pub struct Foo;

impl SayHello for Foo {
    fn say_hello(&self) {}
}

pub struct Bar;

impl SayHello for Bar {
    fn say_hello(&self) {}
}

#[test]
fn test_inherent_pub_crate() {
    let foo = FooOrBar::Foo(Foo);
    foo.say_hello();
}

mod inner {
    disponent::declare!(
        #[disponent::configure(inherent)]
        pub(crate) enum InnerEnum {
            Variant(InnerType),
        }

        pub trait InnerTrait {
            fn inner_method(&self);
        }
    );

    pub struct InnerType;

    impl InnerTrait for InnerType {
        fn inner_method(&self) {}
    }

    pub fn test_inner() {
        let e = InnerEnum::Variant(InnerType);
        e.inner_method();
    }
}

#[test]
fn test_inherent_inherit_visibility() {
    inner::test_inner();
}
