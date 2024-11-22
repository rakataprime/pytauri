use std::ops::Deref as _;

use pyfuture::{future::PyFuture, runner::Runner};
use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::*;
use pyo3::types::{PyCFunction, PyDict, PyModule, PyTuple};
use pytauri_core::tauri_runtime::Runtime;
pub use pytauri_core::{App, AppHandle, RunEvent, RunEventEnum};
use tauri::Context;
use tauri_plugin_pytauri::Commands;

pub(crate) const PYO3_MOD_NAME: &str = "pytauri";

pub fn pymodule_export(
    parent_module: &Bound<'_, PyModule>,
    app_builder: impl Fn(Option<&Bound<'_, PyDict>>) -> PyResult<tauri::Builder<Runtime>>
        + Send
        + 'static,
    context_builder: impl Fn() -> PyResult<Context> + Send + 'static,
) -> PyResult<()> {
    let py = parent_module.py();

    // `args`: tuple[future_runner, Commands]
    // `**kwargs`: Any
    let build_app_closure = move |args: &Bound<'_, PyTuple>,
                                  kwargs: Option<&Bound<'_, PyDict>>|
          -> PyResult<App> {
        let pyfuture_runner = args.get_item(0)?.downcast_into::<Runner>()?.unbind();
        let commands = args
            .get_item(1)?
            .downcast_into::<Commands>()?
            .get()
            .deref()
            .clone();

        let tauri_app = app_builder(kwargs)?
            .plugin(tauri_plugin_pytauri::init(pyfuture_runner, commands))
            .build(context_builder()?)
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to build tauri app: {:?}", e)))?;

        App::try_build(tauri_app).map_err(|_| {
            PyRuntimeError::new_err("An app instance has already been created in this thread")
        })
    };
    let build_app = PyCFunction::new_closure(
        py,
        Some(c"build_app"),
        Some(c"build tauri app"),
        build_app_closure,
    )?;

    let self_module = PyModule::new(py, PYO3_MOD_NAME)?;
    self_module.add_class::<App>()?;
    self_module.add_class::<AppHandle>()?;
    self_module.add_class::<RunEvent>()?;
    self_module.add_class::<RunEventEnum>()?;
    self_module.add_class::<Commands>()?;
    self_module.add_class::<Runner>()?;
    self_module.add_class::<PyFuture>()?;
    self_module.add_function(build_app)?;

    parent_module.add_submodule(&self_module)?;
    Ok(())
}
