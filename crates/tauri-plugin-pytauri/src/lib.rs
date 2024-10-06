mod commands;

use pyo3::prelude::*;
use tauri::plugin::{Builder, TauriPlugin};
use tauri::Runtime;

const PLUGIN_NAME: &str = "pytauri";

pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new(PLUGIN_NAME)
        .invoke_handler(tauri::generate_handler![commands::pyfunc])
        .build()
}

pub fn register_pyo3_module(parent_module: &Bound<'_, PyModule>) -> PyResult<&'static str> {
    let self_module = PyModule::new_bound(parent_module.py(), PLUGIN_NAME)?;
    self_module.add_function(wrap_pyfunction_bound!(
        commands::py_invoke_handler,
        &self_module
    )?)?;
    parent_module.add_submodule(&self_module)?;
    Ok(PLUGIN_NAME)
}
