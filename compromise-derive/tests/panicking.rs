#![cfg(feature = "panicking")]

use compromise::slop;

#[slop]
fn free(value: usize) -> usize;

#[slop]
struct Generic<'a, T, const N: usize>
where
    T: 'a;

#[slop]
impl<'a, T, const N: usize> Generic<'a, T, N>
where
    T: 'a,
{
    fn create(value: T) -> Self;
    fn mutate(&mut self, value: T);
}

trait Factory: Sized {
    fn factory() -> Self;
}

#[slop]
impl<'a, T, const N: usize> Factory for Generic<'a, T, N>
where
    T: 'a,
{
    fn factory() -> Self;
}

fn panic_message(value: Box<dyn std::any::Any + Send>) -> String {
    if let Some(message) = value.downcast_ref::<&str>() {
        (*message).to_owned()
    } else if let Some(message) = value.downcast_ref::<String>() {
        message.clone()
    } else {
        panic!("unexpected panic payload")
    }
}

#[test]
fn free_functions_panic_without_a_backing_module() {
    let panic = std::panic::catch_unwind(|| free(1)).unwrap_err();
    assert_eq!(panic_message(panic), "panicking.free");
}

#[test]
fn generic_structs_and_impls_analyze_without_a_backing_type() {
    assert_eq!(std::mem::size_of::<Generic<'static, String, 4>>(), 0);

    let panic =
        match std::panic::catch_unwind(|| Generic::<'static, String, 4>::create(String::new())) {
            Err(panic) => panic,
            Ok(_) => panic!("generated function unexpectedly returned"),
        };
    assert_eq!(panic_message(panic), "panicking.create");

    let mut value = Generic::<'static, String, 4>(std::marker::PhantomData);
    let panic = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        value.mutate(String::new());
    }))
    .unwrap_err();
    assert_eq!(panic_message(panic), "panicking.mutate");

    let panic = match std::panic::catch_unwind(<Generic<'static, String, 4> as Factory>::factory) {
        Err(panic) => panic,
        Ok(_) => panic!("generated function unexpectedly returned"),
    };
    assert_eq!(panic_message(panic), "panicking.factory");
}
