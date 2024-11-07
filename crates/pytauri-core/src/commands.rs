use anyhow::{anyhow, Context};
use pyfuture::future::{CancelOnDrop, RustFuture};
use pyo3::prelude::*;
use pyo3::types::IntoPyDict as _;
use pyo3::types::PyByteArray;
use tauri::ipc::{InvokeBody, Request, Response};
use tauri::Manager as _;

use crate::pymod::{FutureRunner, PyCommands};
use crate::pytauri::AppHandle;

const PYFUNC_HEADER_KEY: &str = "pyfunc";

async fn invoke_pyfunc(
    request: Request<'_>,
    app_handle: tauri::AppHandle,
) -> anyhow::Result<Response> {
    use dashmap::try_result::TryResult;

    let body = match request.body() {
        InvokeBody::Json(_) => {
            return Err(anyhow!(
                "Please use  `ArrayBuffer` or `Uint8Array` raw request, it's more efficient"
            ))
        }
        InvokeBody::Raw(body) => body,
    };
    let header = request.headers();
    let func_name = header
        .get(PYFUNC_HEADER_KEY)
        .ok_or_else(|| anyhow!("There is no {PYFUNC_HEADER_KEY} header"))
        .context(format!("{header:?}"))?
        .to_str()
        .context("Only support visible ASCII chars")?;

    // Do not use `jiter` to serialize the body into a `PyObject` here, but directly convert it to `PyByteArray`
    //
    // - Flexibility
    //     Users can decide the deserialization scheme on the Python side
    // - Even converting to `byteArray` has very little overhead; the only downside is memory copying
    // - `Pydantic` is quite efficient at deserializing and validating from `byteArray`
    // - Constructing a pydantic model from a `pyobject` that is the result of serialization is very inefficient!
    /*
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
    let future: anyhow::Result<RustFuture> = Python::with_gil(|py| {
        let py_func = {
            let py_commands_state = app_handle.state::<PyCommands>();
            let py_commands_ref = py_commands_state.borrow(py);

            let py_func = match py_commands_ref.handlers.try_get(func_name) {
                TryResult::Present(py_func) => py_func,
                TryResult::Absent => {
                    return Err(anyhow!("The pyfunction `{func_name}` is not registered"))
                }
                TryResult::Locked => {
                    return Err(anyhow!(
                        "The `PY_INVOKE_HANDLERS` is locked, please try later"
                    ))
                }
            };
            py_func.bind(py).clone()
            // drop every references here, especially for `py_commands_ref`
        };

        let func_arg = PyByteArray::new_bound(py, body);
        // TODO, XXX (perf): we create a new PyObject `app_handle_py` every time, which is not efficient;
        // if we can prove that the `app_handle` is singleton, we can cache it(i.g. PyObject).
        // We should create a issue to `tauri`.
        //
        // TODO, XXX (perf): maybe we can cache this `PyDict`, something like `Vec<(PyFunc, PyDict)>`,
        // and determine whether to create `PyClass`(e.g. `app_handle`) by the `PyDict`'s key.
        let app_handle_py = AppHandle(app_handle.clone());
        let func_kwargs = [("app_handle", app_handle_py.into_py(py))].into_py_dict_bound(py);

        let awaitable = py_func
            .call((func_arg,), Some(&func_kwargs))
            .inspect_err(|e| {
                // we also print it to python, instead of raising it to python,
                // that's because python code will not catch the exception, which will make python exit
                e.print_and_set_sys_last_vars(py);
            })
            .context("Failed to call the python function")?;

        let future_runner_state = app_handle.state::<FutureRunner>();
        let future_runner_ref = future_runner_state.borrow(py);
        let future = future_runner_ref.future_bound(py, awaitable.unbind());
        Ok(future)
    });
    let future = future?;

    let future_result = CancelOnDrop(future).await;
    let result = Python::with_gil(|py| {
        match future_result {
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
    })?;

    Ok(Response::new(result))
}

#[tauri::command]
pub(crate) async fn pyfunc(
    request: Request<'_>,
    app_handle: tauri::AppHandle,
) -> Result<Response, String> {
    invoke_pyfunc(request, app_handle)
        .await
        // use `debug` format to display backtrace
        .map_err(|err| format!("{err:?}"))
}
