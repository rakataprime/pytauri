use std::convert::Infallible;
use std::error::Error;
use std::fmt::{Debug, Display, Formatter};
use std::mem::replace;

use parking_lot::{
    MappedRwLockReadGuard, MappedRwLockWriteGuard, RwLock, RwLockReadGuard, RwLockWriteGuard,
};
use pyo3::exceptions::PyRuntimeError;
use pyo3::marker::{Python, Ungil};
use pyo3::PyErr;

const CONSUMED_ERROR_MSG: &str = "Already consumed";
const LOCK_ERROR_MSG: &str = "Already mutably borrowed";

#[derive(Debug)]
pub struct ConsumedError;

impl Display for ConsumedError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{CONSUMED_ERROR_MSG}")
    }
}

impl Error for ConsumedError {}

impl From<ConsumedError> for PyErr {
    fn from(_: ConsumedError) -> Self {
        PyRuntimeError::new_err(CONSUMED_ERROR_MSG)
    }
}

pub type ConsumedResult<T> = Result<T, ConsumedError>;

#[derive(Debug)]
pub struct LockError;

impl Display for LockError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{LOCK_ERROR_MSG}")
    }
}

impl Error for LockError {}

impl From<LockError> for PyErr {
    fn from(_: LockError) -> Self {
        PyRuntimeError::new_err(LOCK_ERROR_MSG)
    }
}

pub type LockResult<T> = Result<T, LockError>;

pub type PyWrapperT0<T> = Result<T, Infallible>;
// TODO, FIXME, PERF: we have to use `RwLock` instead of `RefCell`,
// it's because pyo3 require `Sync`.
//
// We need wait for pyo3 `unsync`, see also:
// - <https://github.com/PyO3/pyo3/issues/4265#issuecomment-2348510879>
// - <https://github.com/pydantic/pydantic-core/pull/1556#issue-2694035224>
//
// ---
//
// use `parking_lot` instead of `std`, it's because `parking_lot` will not poisoned.
pub type PyWrapperT1<T> = RwLock<Result<T, Infallible>>;
pub type PyWrapperT2<T> = RwLock<Result<T, ConsumedError>>;

mod sealed {
    use super::*;

    pub trait PyWrapperT {}

    impl<T> PyWrapperT for PyWrapperT0<T> {}
    impl<T> PyWrapperT for PyWrapperT1<T> {}
    impl<T> PyWrapperT for PyWrapperT2<T> {}

    pub trait SealedPyWrapper {}

    impl<T> SealedPyWrapper for PyWrapper<T> where T: PyWrapperT {}

    pub trait SealedUnsafeUngilExt {}

    impl SealedUnsafeUngilExt for Python<'_> {}
}

trait RwLockExt {
    type T;

    fn try_read_ext(&self) -> LockResult<RwLockReadGuard<'_, Self::T>>;

    fn try_write_ext(&self) -> LockResult<RwLockWriteGuard<'_, Self::T>>;
}

impl<T> RwLockExt for RwLock<T> {
    type T = T;

    fn try_read_ext(&self) -> LockResult<RwLockReadGuard<'_, T>> {
        self.try_read().ok_or(LockError)
    }

    fn try_write_ext(&self) -> LockResult<RwLockWriteGuard<'_, T>> {
        self.try_write().ok_or(LockError)
    }
}

/// NOTE: For [PyWrapper<T>], changes from `T = [PyWrapperT0] -> [PyWrapperT1] -> [PyWrapperT2]`
/// will not be considered breaking changes.
///
/// - When the type is [PyWrapperT0], all methods are zero-cost abstractions.
/// - When the type changes to [PyWrapperT1], compatibility with [PyWrapperT0] is achieved by
///   implicitly calling other methods that acquire locks. These compatible methods will emit
///   deprecation warnings.
/// - When the type changes to [PyWrapperT2], compatibility with [PyWrapperT1] is achieved by
///   implicitly calling [Result::unwrap()] on other methods that return [Result]. These
///   compatible methods will emit deprecation warnings.
pub struct PyWrapper<T>
where
    T: sealed::PyWrapperT,
{
    inner: T,
}

impl<T> PyWrapper<PyWrapperT0<T>> {
    #[inline]
    pub const fn new0(inner: T) -> Self {
        Self { inner: Ok(inner) }
    }

    #[inline]
    pub fn inner_ref(&self) -> impl Deref<Target = T> + '_ {
        // TODO, FIXME: use [Result::into_ok] instead (unstable for now)
        self.inner.as_ref().unwrap()
    }

    #[inline]
    pub fn inner_mut(&mut self) -> impl DerefMut<Target = T> + '_ {
        // TODO, FIXME: use [Result::into_ok] instead (unstable for now)
        self.inner.as_mut().unwrap()
    }

    #[inline]
    pub fn into_inner(self) -> T {
        // TODO, FIXME: use [Result::into_ok] instead (unstable for now)
        self.inner.unwrap()
    }
}

impl<T> PyWrapper<PyWrapperT1<T>> {
    #[inline]
    pub const fn new1(inner: T) -> Self {
        Self {
            inner: RwLock::new(Ok(inner)),
        }
    }

    pub fn lock_inner_ref(&self) -> LockResult<MappedRwLockReadGuard<'_, T>> {
        self.inner
            .try_read_ext()
            // TODO, FIXME: use [Result::into_ok] instead (unstable for now)
            .map(|guard| RwLockReadGuard::map(guard, |inner| inner.as_ref().unwrap()))
    }

    pub fn lock_inner_mut(&self) -> LockResult<MappedRwLockWriteGuard<'_, T>> {
        self.inner
            .try_write_ext()
            // TODO, FIXME: use [Result::into_ok] instead (unstable for now)
            .map(|guard| RwLockWriteGuard::map(guard, |inner| inner.as_mut().unwrap()))
    }

    pub fn into_inner(self) -> T {
        // TODO, FIXME: use [Result::into_ok] instead (unstable for now)
        self.inner.into_inner().unwrap()
    }

    #[deprecated(note = "use `lock_inner_ref` instead")]
    pub fn inner_ref(&self) -> MappedRwLockReadGuard<'_, T> {
        self.lock_inner_ref().expect(LOCK_ERROR_MSG)
    }

    #[deprecated(note = "use `lock_inner_mut` instead")]
    pub fn inner_mut(&self) -> MappedRwLockWriteGuard<'_, T> {
        self.lock_inner_mut().expect(LOCK_ERROR_MSG)
    }
}

impl<T> PyWrapper<PyWrapperT2<T>> {
    pub const fn new2(inner: T) -> Self {
        Self {
            inner: RwLock::new(Ok(inner)),
        }
    }

    pub fn try_lock_inner_ref(&self) -> LockResult<ConsumedResult<MappedRwLockReadGuard<'_, T>>> {
        self.try_read().map(|guard| {
            if guard.is_err() {
                Err(ConsumedError)
            } else {
                // PEFR: it's ok to use [unwrap_unchecked], but i dont like unsafe block
                Ok(RwLockReadGuard::map(guard, |inner| inner.as_ref().unwrap()))
            }
        })
    }

    pub fn try_lock_inner_mut(&self) -> LockResult<ConsumedResult<MappedRwLockWriteGuard<'_, T>>> {
        self.try_write().map(|guard| {
            if guard.is_err() {
                Err(ConsumedError)
            } else {
                // PEFR: it's ok to use [unwrap_unchecked], but i dont like unsafe block
                Ok(RwLockWriteGuard::map(guard, |inner| {
                    inner.as_mut().unwrap()
                }))
            }
        })
    }

    pub fn try_take_inner(&self) -> LockResult<ConsumedResult<T>> {
        self.try_replace_inner(Err(ConsumedError))
    }

    pub fn try_replace_inner(&self, inner: ConsumedResult<T>) -> LockResult<ConsumedResult<T>> {
        self.try_write().map(|mut guard| {
            let result = guard.deref_mut();
            replace(result, inner)
        })
    }

    pub fn try_read(&self) -> LockResult<RwLockReadGuard<'_, ConsumedResult<T>>> {
        self.inner.try_read_ext()
    }

    pub fn try_write(&self) -> LockResult<RwLockWriteGuard<'_, ConsumedResult<T>>> {
        self.inner.try_write_ext()
    }

    pub fn try_into_inner(self) -> ConsumedResult<T> {
        self.inner.into_inner()
    }

    #[deprecated(note = "use `try_lock_inner_ref` instead")]
    pub fn lock_inner_ref(&self) -> LockResult<MappedRwLockReadGuard<'_, T>> {
        self.try_lock_inner_ref()
            .map(|result| result.expect(CONSUMED_ERROR_MSG))
    }

    #[deprecated(note = "use `try_lock_inner_mut` instead")]
    pub fn lock_inner_mut(&self) -> LockResult<MappedRwLockWriteGuard<'_, T>> {
        self.try_lock_inner_mut()
            .map(|result| result.expect(CONSUMED_ERROR_MSG))
    }

    #[deprecated(note = "use `try_lock_inner_ref` instead")]
    pub fn inner_ref(&self) -> MappedRwLockReadGuard<'_, T> {
        self.try_lock_inner_ref()
            .expect(LOCK_ERROR_MSG)
            .expect(CONSUMED_ERROR_MSG)
    }

    #[deprecated(note = "use `try_lock_inner_mut` instead")]
    pub fn inner_mut(&self) -> MappedRwLockWriteGuard<'_, T> {
        self.try_lock_inner_mut()
            .expect(LOCK_ERROR_MSG)
            .expect(CONSUMED_ERROR_MSG)
    }

    #[deprecated(note = "use `try_into_inner` instead")]
    pub fn into_inner(self) -> T {
        self.try_into_inner().expect(CONSUMED_ERROR_MSG)
    }
}

/// You must drop the `T` of [LockResult]::<[ConsumedResult]>::`<T>` to release the potentially acquired lock
pub trait PyWrapperSemverExt: sealed::SealedPyWrapper {
    type Wrapped;

    /// For implementations of [PyWrapper]::<[PyWrapperT1]> and ::<[PyWrapperT2]>, locks will be acquired
    fn inner_ref_semver(&self) -> LockResult<ConsumedResult<impl Deref<Target = Self::Wrapped>>>;
    /// For implementations of [PyWrapper]::<[PyWrapperT1]> and ::<[PyWrapperT2]>, locks will be acquired
    fn inner_mut_semver(
        &mut self,
    ) -> LockResult<ConsumedResult<impl DerefMut<Target = Self::Wrapped>>>;
    fn into_inner_semver(self) -> ConsumedResult<Self::Wrapped>;
}

impl<T> PyWrapperSemverExt for PyWrapper<PyWrapperT0<T>> {
    type Wrapped = T;

    fn inner_ref_semver(&self) -> LockResult<ConsumedResult<impl Deref<Target = Self::Wrapped>>> {
        Ok(Ok(self.inner_ref()))
    }

    fn inner_mut_semver(
        &mut self,
    ) -> LockResult<ConsumedResult<impl DerefMut<Target = Self::Wrapped>>> {
        Ok(Ok(self.inner_mut()))
    }

    fn into_inner_semver(self) -> ConsumedResult<Self::Wrapped> {
        Ok(self.into_inner())
    }
}

impl<T> PyWrapperSemverExt for PyWrapper<PyWrapperT1<T>> {
    type Wrapped = T;

    fn inner_ref_semver(&self) -> LockResult<ConsumedResult<impl Deref<Target = Self::Wrapped>>> {
        self.lock_inner_ref().map(Ok)
    }

    fn inner_mut_semver(
        &mut self,
    ) -> LockResult<ConsumedResult<impl DerefMut<Target = Self::Wrapped>>> {
        self.lock_inner_mut().map(Ok)
    }

    fn into_inner_semver(self) -> ConsumedResult<Self::Wrapped> {
        Ok(self.into_inner())
    }
}

impl<T> PyWrapperSemverExt for PyWrapper<PyWrapperT2<T>> {
    type Wrapped = T;

    fn inner_ref_semver(&self) -> LockResult<ConsumedResult<impl Deref<Target = Self::Wrapped>>> {
        self.try_lock_inner_ref()
    }

    fn inner_mut_semver(
        &mut self,
    ) -> LockResult<ConsumedResult<impl DerefMut<Target = Self::Wrapped>>> {
        self.try_lock_inner_mut()
    }

    fn into_inner_semver(self) -> ConsumedResult<Self::Wrapped> {
        self.try_into_inner()
    }
}

/// Since [PyClass] does not support lifetimes, in such cases, we will declare a [PyClass(Py<T>)],
/// where a method of `T` can obtain a `&Ref`, and we will implement [Deref<Target=Ref>] for this [PyClass].
pub use std::ops::{Deref, DerefMut};

/// In the future, we will provide a macro for this trait to automatically generate pymethod
pub trait PyMatchRef {
    type Output;

    fn match_ref(&self) -> Self::Output;
}

/// In the future, we will provide a macro for this trait to automatically generate pymethod
pub trait PyMatchMut {
    type Output;

    fn match_mut(&mut self) -> Self::Output;
}

/// In the future, we will provide a macro for this trait to automatically generate pymethod
pub trait PyMatchInto {
    type Output;

    fn match_into(self) -> Self::Output;
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
    /// you ensure that [ungil] does not hold the GIL.
    ///
    /// See also:
    /// - https://docs.rs/pyo3/0.23.2/pyo3/marker/index.html
    /// - https://docs.rs/pyo3/0.23.2/pyo3/marker/trait.Ungil.html
    /// - https://github.com/PyO3/pyo3/issues/2141
    ///
    /// If you want to bypass multiple `!`[Send] types simultaneously, you can pass them as a `tuple`
    ///
    /// # Safety
    ///
    /// You must ensure that [ungil] does not hold the GIL, such as these types:
    /// <https://docs.rs/pyo3/0.23.2/pyo3/marker/index.html#a-proper-implementation-using-an-auto-trait>
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
