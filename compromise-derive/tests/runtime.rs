#![cfg(not(feature = "panicking"))]

use compromise::{FromSlop, IntoSlop, slop};

mod zz_slop {
    pub mod runtime {
        #[derive(Debug, PartialEq)]
        pub struct Container<T>(pub T);

        pub fn free(left: usize, right: usize) -> usize {
            left * 10 + right
        }

        pub const fn constant(value: usize) -> usize {
            value + 1
        }

        pub async fn asynchronous(value: usize) -> usize {
            value + 2
        }

        pub unsafe fn dangerous(value: usize) -> usize {
            value + 3
        }

        pub fn create<T>(value: T) -> Container<T> {
            Container(value)
        }

        pub fn value<T>(value: &Container<T>) -> &T {
            &value.0
        }

        pub fn replace<T>(value: &mut Container<T>, replacement: T) {
            value.0 = replacement;
        }

        pub fn consume<T>(value: Container<T>) -> T {
            value.0
        }

        pub fn nested<T>(value: T) -> Option<Result<Vec<Box<Container<T>>>, &'static str>> {
            Some(Ok(vec![Box::new(Container(value))]))
        }

        pub fn combine<T: Clone>(value: &Container<T>, other: &Container<T>) -> Container<T> {
            let _ = value;
            Container(other.0.clone())
        }

        pub fn transform<T>(value: Option<Container<T>>) -> Option<Container<T>> {
            value
        }
    }
}

mod nested {
    use super::slop;

    #[slop]
    pub fn free(left: usize, right: usize) -> usize;

    #[slop]
    pub const fn constant(value: usize) -> usize;

    #[slop]
    pub async fn asynchronous(value: usize) -> usize;

    #[slop]
    pub unsafe fn dangerous(value: usize) -> usize;

    #[slop]
    #[derive(Debug, PartialEq)]
    pub struct Container<T>
    where
        T: Clone;

    #[slop]
    impl<T> Container<T>
    where
        T: Clone,
    {
        pub const LABEL: &'static str = "container";

        pub fn create(value: T) -> Self;
        pub fn value(&self) -> &T;
        pub fn replace(&mut self, replacement: T);
        pub fn consume(self) -> T;
        pub fn nested(value: T) -> Option<Result<Vec<Box<Self>>, &'static str>>;
    }

    pub trait Example: Sized {
        type Value;

        fn combine(&self, other: &Self) -> Self;
        fn transform(value: Option<Self>) -> Option<Self>;
    }

    #[slop]
    impl<T> Example for Container<T>
    where
        T: Clone,
    {
        type Value = T;

        fn combine(&self, other: &Self) -> Self;
        fn transform(value: Option<Self>) -> Option<Self>;
    }
}

#[test]
fn free_function_delegates_with_all_arguments() {
    assert_eq!(nested::free(4, 2), 42);
    const VALUE: usize = nested::constant(4);
    assert_eq!(VALUE, 5);

    let mut future = std::pin::pin!(nested::asynchronous(4));
    let mut context = std::task::Context::from_waker(std::task::Waker::noop());
    assert_eq!(
        std::future::Future::poll(future.as_mut(), &mut context),
        std::task::Poll::Ready(6)
    );

    // SAFETY: the delegated test function has no additional preconditions.
    assert_eq!(unsafe { nested::dangerous(4) }, 7);
}

#[test]
fn inherent_methods_delegate_and_preserve_other_items() {
    assert_eq!(nested::Container::<String>::LABEL, "container");

    let mut container = nested::Container::create("first".to_owned());
    assert_eq!(container.value(), "first");
    container.replace("second".to_owned());
    assert_eq!(container.consume(), "second");
}

#[test]
fn nested_self_returns_use_from_slop_recursively() {
    let nested = nested::Container::nested("value".to_owned())
        .unwrap()
        .unwrap();
    assert_eq!(nested[0].value(), "value");
}

#[test]
fn trait_impl_converts_receivers_arguments_and_returns() {
    use nested::Example;

    let left = nested::Container::create("left".to_owned());
    let right = nested::Container::create("right".to_owned());
    assert_eq!(left.combine(&right).value(), "right");

    let transformed = nested::Container::transform(Some(right)).unwrap();
    assert_eq!(transformed.value(), "right");
}

#[test]
fn generated_conversion_traits_expose_the_private_representation_safely() {
    let public = nested::Container::from_slop(zz_slop::runtime::Container("value".to_owned()));
    let inner: zz_slop::runtime::Container<String> = public.into_slop();
    assert_eq!(inner.0, "value");
}
