use std::error::Error;
use std::fmt::{Display, Formatter};

use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::*;

/// Utility for converting [tauri::Error] to [pyo3::PyErr].
///
/// See also: <https://pyo3.rs/v0.23.2/function/error-handling.html#foreign-rust-error-types>.
///
/// # Example
///
/**
```rust
use pyo3::prelude::*;
use pytauri_core::utils::{TauriError, TauriResult};

fn tauri_result() -> tauri::Result<()> {
    Ok(())
}

#[pyfunction]
fn foo() -> PyResult<()> {
    tauri_result().map_err(Into::<TauriError>::into)?;
    Ok(())
}

#[pyfunction]
fn bar() -> TauriResult<()> {
    tauri_result()?;
    Ok(())
}
```
*/

#[derive(Debug)]
pub struct TauriError(tauri::Error);

impl Display for TauriError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:}", self.0)
    }
}

impl Error for TauriError {}

impl From<TauriError> for PyErr {
    fn from(value: TauriError) -> Self {
        PyRuntimeError::new_err(format!("{:?}", value.0))
    }
}

impl From<tauri::Error> for TauriError {
    fn from(value: tauri::Error) -> Self {
        Self(value)
    }
}

pub type TauriResult<T> = Result<T, TauriError>;
