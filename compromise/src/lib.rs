//! Forward declarations backed by generated implementations in `crate::zz_slop`.

extern crate self as compromise;

pub use compromise_derive::slop;

/// Converts a value from its `zz_slop` representation.
pub trait FromSlop<T> {
    fn from_slop(value: T) -> Self;
}

/// Converts a value into its `zz_slop` representation.
pub trait IntoSlop<T> {
    fn into_slop(self) -> T;
}

impl<S, T> FromSlop<Option<S>> for Option<T>
where
    T: FromSlop<S>,
{
    fn from_slop(value: Option<S>) -> Self {
        value.map(T::from_slop)
    }
}

impl<S, T> IntoSlop<Option<T>> for Option<S>
where
    S: IntoSlop<T>,
{
    fn into_slop(self) -> Option<T> {
        self.map(S::into_slop)
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

impl<S, T, E> IntoSlop<Result<T, E>> for Result<S, E>
where
    S: IntoSlop<T>,
{
    fn into_slop(self) -> Result<T, E> {
        self.map(S::into_slop)
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

impl<S, T> IntoSlop<Vec<T>> for Vec<S>
where
    S: IntoSlop<T>,
{
    fn into_slop(self) -> Vec<T> {
        self.into_iter().map(S::into_slop).collect()
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

impl<S, T> IntoSlop<Box<T>> for Box<S>
where
    S: IntoSlop<T>,
{
    fn into_slop(self) -> Box<T> {
        Box::new(S::into_slop(*self))
    }
}
