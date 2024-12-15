/// TODO: In the future, we will provide a macro for this trait to automatically generate pymethod
pub trait PyMatchRef {
    type Output;

    fn match_ref(&self) -> Self::Output;
}

/// TODO: In the future, we will provide a macro for this trait to automatically generate pymethod
pub trait PyMatchMut {
    type Output;

    fn match_mut(&mut self) -> Self::Output;
}

/// TODO: In the future, we will provide a macro for this trait to automatically generate pymethod
///
/// It is recommended to implement this trait only when using `clone` in [PyMatchRef]/[PyMatchMut]
/// would significantly impact memory/performance.
pub trait PyMatchInto {
    type Output;

    fn match_into(self) -> Self::Output;
}
