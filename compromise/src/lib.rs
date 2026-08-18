//! Forward declarations backed by generated implementations in `crate::zz_slop`.

extern crate self as compromise;

pub use compromise_derive::slop;

pub trait FromSlop<T> {
    fn from_slop(value: T) -> Self;
}

pub trait IntoSlop<T> {
    fn into_slop(self) -> T;
}

impl<S, T> IntoSlop<T> for S
where
    T: FromSlop<S>,
{
    fn into_slop(self) -> T {
        T::from_slop(self)
    }
}

impl<S, T> FromSlop<Option<S>> for Option<T>
where
    T: FromSlop<S>,
{
    fn from_slop(value: Option<S>) -> Self {
        value.map(T::from_slop)
    }
}

impl<S, T, E> FromSlop<Result<S, E>> for Result<T, E>
where
    T: FromSlop<S>,
{
    fn from_slop(value: Result<S, E>) -> Self {
        value.map(T::from_slop)
    }
}

impl<S, T> FromSlop<Vec<S>> for Vec<T>
where
    T: FromSlop<S>,
{
    fn from_slop(value: Vec<S>) -> Self {
        value.into_iter().map(T::from_slop).collect()
    }
}

impl<S, T> FromSlop<Box<S>> for Box<T>
where
    T: FromSlop<S>,
{
    fn from_slop(value: Box<S>) -> Self {
        Box::new(T::from_slop(*value))
    }
}

impl<S, T, const N: usize> FromSlop<[S; N]> for [T; N]
where
    T: FromSlop<S>,
{
    fn from_slop(value: [S; N]) -> Self {
        value.map(T::from_slop)
    }
}
