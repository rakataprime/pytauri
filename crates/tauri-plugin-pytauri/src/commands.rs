use dashmap::DashMap;
use std::sync::LazyLock;

use anyhow::{anyhow, Context};
use pyo3::exceptions as py_exceptions;
use pyo3::prelude::*;
use pyo3::types as py_types;
use tauri::ipc::{InvokeBody, Request, Response};

const PYFUNC_HEADER_KEY: &str = "pyfunc";

static PY_INVOKE_HANDLERS: LazyLock<DashMap<String, Py<PyAny>>> = LazyLock::new(DashMap::new);

fn invoke_pyfunc(request: Request) -> anyhow::Result<Response> {
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

    let py_func = match PY_INVOKE_HANDLERS.try_get(func_name) {
        TryResult::Present(py_func) => py_func,
        TryResult::Absent => return Err(anyhow!("The pyfunction `{func_name}` is not registered")),
        TryResult::Locked => {
            return Err(anyhow!(
                "The `PY_INVOKE_HANDLERS` is locked, please try later"
            ))
        }
    };

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

    let invoke_return: anyhow::Result<Vec<u8>> = Python::with_gil(|py| {
        let func_arg = py_types::PyByteArray::new_bound(py, body);
        let invoke_return = py_func
            .bind(py)
            .call1((func_arg,))
            .context("Failed to call the python function")?
            // [`Response`] only accepts [`Vec<u8>`] as input,
            .extract::<Vec<u8>>()
            .context("The python function should return a variable which is not bytes-like")?;
        Ok(invoke_return)
    });

    Ok(Response::new(invoke_return?))
}

#[tauri::command]
pub(crate) fn pyfunc(request: Request) -> Result<Response, String> {
    invoke_pyfunc(request)
        // use `debug` format to display backtrace
        .map_err(|err| format!("{err:?}"))
}

/// Register a python function to be called from Rust.
#[pyfunction]
pub(crate) fn py_invoke_handler(func_name: String, py_func: Bound<'_, PyAny>) -> PyResult<()> {
    use dashmap::Entry;
    use py_exceptions::{PyRuntimeError, PyValueError};

    // We only check once when the first time adding the handler,
    // so the cost of checking is acceptable.
    if !py_func.is_callable() {
        return Err(PyValueError::new_err("The object is not callable"));
    }

    let entry = PY_INVOKE_HANDLERS
        .try_entry(func_name)
        .ok_or(PyRuntimeError::new_err(
            "More than one thread is trying to register the invoke handler",
        ))?;

    match entry {
        Entry::Occupied(_) => return Err(PyValueError::new_err("Function name already exists")),
        Entry::Vacant(vacant) => {
            vacant.insert(py_func.unbind());
        }
    };

    Ok(())
}
