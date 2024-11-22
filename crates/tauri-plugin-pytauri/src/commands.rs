use std::ops::{Deref, DerefMut};
use std::sync::Arc;
use std::sync::LazyLock;

use anyhow::{anyhow, Context as _};
use dashmap::DashMap;
use http::header::HeaderMap;
use pyfuture::future::{CancelOnDrop, RustFuture};
use pyfuture::runner::Runner;
use pyo3::exceptions::{PyRuntimeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{IntoPyDict as _, PyByteArray};
pub(crate) use pytauri_core::tauri_runtime::Runtime as PyTauriRuntime;
use pytauri_core::AppHandle;
use tauri::ipc::{Invoke, InvokeBody, InvokeError, InvokeResponseBody};
use tauri::Manager as _;
use tokio::runtime as rt;

use crate::PyTauriExt as _;

pub struct CommandsInner {
    handlers: DashMap<String, PyObject>,
}

// [pymethods]
impl CommandsInner {
    fn new() -> Self {
        Self {
            handlers: DashMap::new(),
        }
    }

    /// Register a async python function to be called from Rust.
    /// `py_func`: Callable[..., Awaitable[bytes]], see `invoke_pyfunc` implementation for `...`
    fn invoke_handler(&self, func_name: String, py_func: PyObject) -> PyResult<()> {
        use dashmap::Entry;

        let Self { handlers, .. } = self;

        // TODO (perf): I don't know if we need to use `py.allow_threads` here,
        // inserting a new entry into the `DashMap` seems to be a short operation.
        {
            let entry = handlers
                .try_entry(func_name)
                .ok_or(PyRuntimeError::new_err(
                    "More than one thread is trying to register the invoke handler",
                ))?;

            match entry {
                Entry::Occupied(_) => {
                    return Err(PyValueError::new_err("Function name already exists"))
                }
                Entry::Vacant(vacant) => {
                    vacant.insert(py_func);
                }
            };

            Ok(())
        }
    }
}

// [rust methods]
impl CommandsInner {
    const PYFUNC_HEADER_KEY: &str = "pyfunc";

    #[inline]
    fn get_raw_from_invoke_body(invoke_body: &InvokeBody) -> anyhow::Result<&Vec<u8>> {
        match invoke_body {
            InvokeBody::Json(_) => Err(anyhow!(
                "Please use  `ArrayBuffer` or `Uint8Array` raw request, it's more efficient"
            )),
            InvokeBody::Raw(body) => Ok(body),
        }
    }

    #[inline]
    fn get_func_name_from_headers(headers: &HeaderMap) -> anyhow::Result<&str> {
        const PYFUNC_HEADER_KEY: &str = CommandsInner::PYFUNC_HEADER_KEY;

        let func_name = headers
            .get(PYFUNC_HEADER_KEY)
            .ok_or_else(|| anyhow!("There is no {PYFUNC_HEADER_KEY} header"))
            .context(format!("{headers:?}"))?
            .to_str()
            .context("Only support visible ASCII chars")?;
        Ok(func_name)
    }

    #[inline]
    fn get_py_func<'a>(
        &'a self,
        func_name: &str,
    ) -> anyhow::Result<dashmap::mapref::one::Ref<'a, String, PyObject>> {
        use dashmap::try_result::TryResult;

        let py_func = match self.handlers.try_get(func_name) {
            TryResult::Present(py_func) => Ok(py_func),
            TryResult::Absent => Err(anyhow!("The pyfunction `{func_name}` is not registered")),
            TryResult::Locked => Err(anyhow!(
                "The `PY_INVOKE_HANDLERS` is locked, please try later"
            )),
        };
        py_func
    }

    fn invoke_pyfunc(
        py: Python<'_>,
        pyfuture_runner: &Py<Runner>,
        py_func: &PyObject,
        json_data: &[u8],
        app_handle: tauri::AppHandle<PyTauriRuntime>,
    ) -> anyhow::Result<RustFuture> {
        /*
        Do not use `jiter` to serialize the body into a `PyObject` here, but directly convert it to `PyByteArray`

        - Flexibility
            Users can decide the deserialization scheme on the Python side
        - Even converting to `byteArray` has very little overhead; the only downside is memory copying
        - `Pydantic` is quite efficient at deserializing and validating from `byteArray`
        - Constructing a pydantic model from a `pyobject` that is the result of serialization is very inefficient!

        ## benchmark

        ```console
        ########## bytes
        Number of iterations: 100000
        get_pybytes     : 0.0078 seconds
        ########## py obj
        Number of iterations: 100000
        rust_serde      : 0.0484 seconds
        rust_serde_from_pybytes : 0.0636 seconds
        py_serde_from_pybytes   : 0.2405 seconds
        ########## pydantic
        Number of iterations: 100000
        pydantic_serde_and_validate_from_pybytes        : 0.1736 seconds
        pydantic_validate       : 0.1868 seconds
        pydantic_construct      : 0.3021 seconds
        ```
        */

        let func_arg = PyByteArray::new(py, json_data);
        // TODO, XXX (perf): we create a new PyObject `app_handle_py` every time, which is not efficient;
        // if we can prove that the `app_handle` is singleton, we can cache it(i.g. PyObject).
        // We should create a issue to `tauri`.
        //
        // TODO, XXX (perf): maybe we can cache this `PyDict`, something like `Vec<(PyFunc, PyDict)>`,
        // and determine whether to create `PyClass`(e.g. `app_handle`) by the `PyDict`'s key.
        let app_handle_py = AppHandle::new(app_handle);
        let func_kwargs = [(
            "app_handle",
            app_handle_py
                .into_pyobject(py)
                // it should not panic
                .expect("failed to create pyobject"),
        )]
        .into_py_dict(py)
        // it should not panic
        .expect("failed to create pyobject");

        let py_func = py_func.bind(py);
        let awaitable = py_func
            .call((func_arg,), Some(&func_kwargs))
            .inspect_err(|e| {
                // we also print it to python, instead of raising it to python,
                // that's because python code will not catch the exception, which will make python exit
                e.print_and_set_sys_last_vars(py);
            })
            .context("Failed to call the python function")?;

        let future_runner_ref = pyfuture_runner.borrow(py);
        let future = future_runner_ref
            .try_future(py, awaitable.unbind())
            .ok_or(
                anyhow!("python future runner is already closed, all python async tasks will fail")
                    .context(format!("runner: {pyfuture_runner:?}")),
            )?;
        Ok(future)
    }

    fn extract_vec_u8_from_result(
        py: Python<'_>,
        result: &PyResult<PyObject>,
    ) -> anyhow::Result<Vec<u8>> {
        match result {
            Ok(result) => {
                // [`Response`] only accepts [`Vec<u8>`] as input,
                result.extract::<Vec<u8>>(py).map_err(|e| {
                    anyhow!("{e:?}: The python awaitable return a variable which is not bytes-like")
                })
            }
            Err(e) => {
                // we also print it to python, instead of raising it to python,
                // that's because python code will not catch the exception, which will make python exit
                e.print_and_set_sys_last_vars(py);
                Err(anyhow!("{e:?}: Failed to run python awaitable"))
            }
        }
    }
}

// `subclass` for python can override the `invoke_handler` method
#[pyclass(subclass, frozen)]
pub struct Commands(Arc<CommandsInner>);

#[pymethods]
impl Commands {
    #[new]
    #[inline]
    fn new() -> Self {
        Self(Arc::new(CommandsInner::new()))
    }

    #[inline]
    fn invoke_handler(&self, func_name: String, py_func: PyObject) -> PyResult<()> {
        self.0.invoke_handler(func_name, py_func)
    }
}

impl Deref for Commands {
    type Target = Arc<CommandsInner>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Commands {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Commands {
    pub fn into_inner(self) -> Arc<CommandsInner> {
        self.0
    }
}

/// This runtime is specifically for [std::future::Future] that requires acquiring the GIL
static PY_FUTURE_RUNTIME: LazyLock<rt::Runtime> = LazyLock::new(|| {
    // When scheduling Python future, the GIL lock is needed almost the entire time,
    // so multithreading is meaningless
    const WORKER_THREADS: usize = 1;
    let thread_name = format!("{}-pyfuture-rt", env!("CARGO_PKG_NAME"));

    let runtime = rt::Builder::new_multi_thread()
        .enable_all()
        .worker_threads(WORKER_THREADS)
        .thread_name(&thread_name)
        .build()
        .unwrap_or_else(|_| panic!("Failed to create the `{thread_name}` runtime"));
    runtime
});

/// # NOTE
///
/// This function should run in the dedicated runtime [PY_FUTURE_RUNTIME] because it acquires the GIL
async fn _invoke_pyfunc_cmd_with_gil(invoke: &Invoke<PyTauriRuntime>) -> anyhow::Result<Vec<u8>> {
    let Invoke { message, .. } = invoke;
    let app_handle = message.webview_ref().app_handle();
    let func_name = CommandsInner::get_func_name_from_headers(message.headers())?;
    let json_data = CommandsInner::get_raw_from_invoke_body(message.payload())?;

    let pycommands = app_handle.pycommands();
    let py_func = pycommands.get_py_func(func_name)?;
    let pyfuture_runner = app_handle.pyfuture_runner();

    let invoke_future = Python::with_gil(|py| {
        CommandsInner::invoke_pyfunc(
            py,
            &pyfuture_runner,
            &py_func,
            json_data,
            app_handle.clone(),
        )
    })?;

    let invoke_result = CancelOnDrop(invoke_future).await;

    let response =
        Python::with_gil(|py| CommandsInner::extract_vec_u8_from_result(py, &invoke_result))?;

    Ok(response)
}

/// # NOTE
///
/// This function should run in the dedicated runtime [PY_FUTURE_RUNTIME] because it acquires the GIL
async fn invoke_pyfunc_cmd_with_gil(invoke: Invoke<PyTauriRuntime>) {
    let response = _invoke_pyfunc_cmd_with_gil(&invoke)
        .await
        .map(InvokeResponseBody::Raw)
        .map_err(InvokeError::from_anyhow);
    let Invoke { resolver, .. } = invoke;
    resolver.respond(response);
}

fn pyfunc(invoke: Invoke<PyTauriRuntime>) {
    PY_FUTURE_RUNTIME.spawn(invoke_pyfunc_cmd_with_gil(invoke));
}

pub(crate) fn invoke_handler(invoke: Invoke<PyTauriRuntime>) -> bool {
    let invoke_cmd = invoke.message.command().to_owned();
    match invoke_cmd.as_str() {
        "pyfunc" => {
            pyfunc(invoke);
            true
        }
        _ => false,
    }
}
