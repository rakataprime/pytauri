//! This module allows you to use `!`[Send] types in [Python::allow_threads].

use pyo3::marker::{Python, Ungil};

mod sealed {
    use super::*;

    pub trait SealedUnsafeUngilExt {}

    impl SealedUnsafeUngilExt for Python<'_> {}
}

// Do not expose this type to users to prevent them from mistakenly using [Send] trait;
// it should only be used within [UnsafeUngilExt::unsafe_allow_threads]
#[non_exhaustive]
struct UnsafeUngil<T>(T);

// pyo3 will automatically implement `Ungil` for `T: Send`
unsafe impl<T> Send for UnsafeUngil<T> {}

impl<T> UnsafeUngil<T> {
    pub const unsafe fn new(value: T) -> Self {
        Self(value)
    }
}

pub trait UnsafeUngilExt: sealed::SealedUnsafeUngilExt {
    /// pyo3 uses [Send] to implement [pyo3::marker::Ungil], but this is not entirely reasonable.
    /// It prevents some types that are not [Send] but do not actually hold the GIL (e.g., [std::rc::Rc]).
    /// [UnsafeUngilExt::allow_threads_unsend] allows you to bypass this restriction as long as
    /// you ensure that `ungil` does not hold the GIL.
    ///
    /// See also:
    ///
    /// - <https://docs.rs/pyo3/0.23.2/pyo3/marker/index.html>
    /// - <https://docs.rs/pyo3/0.23.2/pyo3/marker/trait.Ungil.html>
    /// - <https://github.com/PyO3/pyo3/issues/2141>
    ///
    /// If you want to bypass multiple `!`[Send] types simultaneously, you can pass them as a `tuple`
    ///
    /// # Safety
    ///
    /// You must ensure that `ungil` does not hold the GIL, such as these types:
    /// <https://docs.rs/pyo3/0.23.2/pyo3/marker/index.html#a-proper-implementation-using-an-auto-trait>
    ///
    /// # Example
    ///
    /**
    ```rust
    use std::rc::Rc;

    use pyo3::prelude::*;
    use pyo3_utils::ungil::UnsafeUngilExt;

    fn foo(py: Python<'_>) {
        let rc = Rc::new(42);
        let rc_ref = &rc;

        unsafe {
            py.allow_threads_unsend(rc_ref, |rc_ref| {
                let _ = rc_ref.clone();
            });
        }
    }
    ```
    */
    unsafe fn allow_threads_unsend<T, F, U>(self, ungil: U, f: F) -> T
    where
        F: Ungil + FnOnce(U) -> T + Send,
        T: Ungil;
}

impl UnsafeUngilExt for Python<'_> {
    unsafe fn allow_threads_unsend<T, F, U>(self, ungil: U, f: F) -> T
    where
        F: Ungil + FnOnce(U) -> T + Send,
        T: Ungil,
    {
        let unsafe_ungil = UnsafeUngil::new(ungil);
        self.allow_threads(move || {
            let unsafe_ungil = unsafe_ungil;
            f(unsafe_ungil.0)
        })
    }
}
