use pyo3::prelude::*;

use crate::future::RustFuture;

#[cfg(not(feature = "sync"))]
use std::marker::PhantomData;
#[cfg(feature = "sync")]
use {
    std::sync::Arc,
    tokio::sync::{OwnedRwLockWriteGuard, RwLock},
};

type Empty = ();

#[derive(Debug)]
struct PyRunner<T> {
    inner: PyObject,
    #[cfg(feature = "sync")]
    _alive_guard: OwnedRwLockWriteGuard<T>,
    #[cfg(not(feature = "sync"))]
    _alive_guard: PhantomData<T>,
}

#[derive(Debug)]
enum RunnerInner {
    Alive {
        runner: PyRunner<Empty>,
        #[cfg(feature = "sync")]
        alive_lock: Arc<RwLock<Empty>>,
    },
    Closed,
}

#[derive(Debug)]
#[pyclass(weakref)]
pub struct Runner(RunnerInner);

#[pymethods]
impl Runner {
    #[new]
    fn new(runner: PyObject) -> Self {
        #[cfg(feature = "sync")]
        {
            let alive_lock = Arc::new(RwLock::new(()));
            // never panic for `unwrap`, because we just created the lock now,
            // no other are writing to it.
            let alive_guard = alive_lock.clone().try_write_owned().unwrap();

            let runner = PyRunner {
                inner: runner,
                _alive_guard: alive_guard,
            };
            Self(RunnerInner::Alive { runner, alive_lock })
        }
        #[cfg(not(feature = "sync"))]
        {
            let runner = PyRunner {
                inner: runner,
                _alive_guard: PhantomData,
            };
            Self(RunnerInner::Alive { runner })
        }
    }

    fn close(&mut self) {
        // drop the `OwnedRwLockWriteGuard`
        // then other `Arc<RwLock<()>>` can use `read` to know the runner is closed.
        if let RunnerInner::Alive { .. } = self.0 {
            self.0 = RunnerInner::Closed;
        }
    }
}

impl Runner {
    pub fn try_future(&self, py: Python<'_>, awaitable: PyObject) -> Option<RustFuture> {
        match &self.0 {
            RunnerInner::Alive { runner, .. } => {
                let runner = runner.inner.clone_ref(py);
                Some(RustFuture::new(runner, awaitable))
            }
            RunnerInner::Closed => None,
        }
    }
    pub fn future(&self, py: Python<'_>, awaitable: PyObject) -> RustFuture {
        self.try_future(py, awaitable)
            .expect("The runner is already closed")
    }

    pub fn is_closed(&self) -> bool {
        matches!(self.0, RunnerInner::Closed)
    }
}

#[cfg(feature = "sync")]
pub use notification::*;
#[cfg(feature = "sync")]
mod notification {
    use super::*;

    impl Runner {
        pub fn closed_notificator(&self) -> Option<ClosedNotificator> {
            match &self.0 {
                RunnerInner::Alive { alive_lock, .. } => {
                    Some(ClosedNotificator::new(alive_lock.clone()))
                }
                RunnerInner::Closed => None,
            }
        }
    }

    // NOTE: We do not derive `Clone` for now,
    // although we could, but I don't see any point in doing so.
    // If you indeed need `Clone`, issue/PR is welcome.
    #[derive(Debug)]
    pub struct ClosedNotificator(Arc<RwLock<Empty>>);

    // if we can get `RwLockReadGuard`, it means
    // the `Runner` has already been called `close` method.
    // That's because `close` method will drop the `OwnedRwLockWriteGuard` of `Runner`,
    //
    // NOTE: DO NOT write the `RwLock`!
    impl ClosedNotificator {
        const fn new(alive_lock: Arc<RwLock<Empty>>) -> Self {
            Self(alive_lock)
        }

        pub fn is_closed(&self) -> bool {
            self.0.try_read().is_ok()
        }

        pub async fn wait(&self) {
            let _ = self.0.read().await;
        }

        pub fn blocking_wait(&self) {
            let _ = self.0.blocking_read();
        }
    }
}
