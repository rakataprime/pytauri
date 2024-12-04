use std::sync::LazyLock;

use pyo3::prelude::*;

use tokio::runtime as rt;
use tokio::task::JoinHandle;

/// This runtime is specifically for [std::future::Future] that requires acquiring the GIL
pub(crate) static GIL_RUNTIME: LazyLock<rt::Runtime> = LazyLock::new(|| {
    let mut builder = rt::Builder::new_multi_thread();
    // https://pyo3.rs/v0.23.2/free-threading.html?highlight=Py_GIL_DISABLED#supporting-free-threaded-python-with-pyo3
    #[cfg(not(Py_GIL_DISABLED))]
    {
        // For non-Free-Threaded CPython, only one thread can acquire the GIL at a time,
        // so multithreading runtime is meaningless
        builder.worker_threads(1);
    }

    let thread_name = format!("{}-gil-rt", env!("CARGO_PKG_NAME"));
    let runtime = builder
        .enable_all()
        .thread_name(&thread_name)
        .build()
        .unwrap_or_else(|e| panic!("Failed to create the `{thread_name}` tokio runtime: {e}"));
    runtime
});

/// **If built for `#[cfg(not(Py_GIL_DISABLED))]`, please NOTE**:
///
/// Do not use [Python::allow_threads] to temporarily release the GIL in a task,
/// because reacquiring the GIL will block the tokio runtime.
/// If you really need to release the GIL, use **another** tokio runtime to spawn
/// a new task that does not require the GIL within the GIL task, and immediately end the GIL task;
/// when you need the GIL again, simply call [task_with_gil] to reacquire the GIL.
///
/// > - If you do not use **another** tokio runtime to spawn new tasks,
/// >     then when the GIL task in [GIL_RUNTIME] is running,
/// >     non-GIL tasks will also be blocked.
///
/// In short, when `#[cfg(not(Py_GIL_DISABLED))]` is true,
/// this runtime can only be used for tasks that hold the GIL for their entire duration.
// TODO, PERF: Use a queue to store tasks, and schedule multiple tasks at once when acquiring the GIL
pub(crate) fn task_with_gil<F, R>(f: F) -> JoinHandle<R>
where
    F: for<'py> FnOnce(Python<'py>) -> R + Send + 'static,
    R: Send + 'static,
{
    let future = async move { Python::with_gil(f) };
    GIL_RUNTIME.spawn(future)
}
