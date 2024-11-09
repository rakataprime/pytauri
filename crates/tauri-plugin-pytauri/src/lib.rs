mod commands;

use std::ops::Deref;
use std::sync::Arc;

use pyfuture::runner::Runner;
use pyo3::prelude::*;
use tauri::plugin::{Builder, TauriPlugin};
use tauri::{Manager, Runtime, State};

use crate::commands::{invoke_handler, PyTauriRuntime};
pub use crate::commands::{Commands, CommandsInner};

const PLUGIN_NAME: &str = "pytauri";

type PyFutureRunnerInner = Py<Runner>;
struct PyFutureRunner(PyFutureRunnerInner);

type PyCommandsInner = Arc<CommandsInner>;
struct PyCommands(PyCommandsInner);

pub fn init(
    pyfuture_runner: PyFutureRunnerInner,
    commands: PyCommandsInner,
) -> TauriPlugin<PyTauriRuntime> {
    Builder::<PyTauriRuntime>::new(PLUGIN_NAME)
        .invoke_handler(invoke_handler)
        .setup(|app_handle, _plugin_api| {
            // if false, there has already state set for the app instance.
            if !app_handle.manage(PyFutureRunner(pyfuture_runner)) {
                unreachable!(
                    "`PyFutureRunner` is private, so it is impossible for other crates to manage it"
                )
            }
            if !app_handle.manage(PyCommands(commands)) {
                unreachable!(
                    "`PyCommands` is private, so it is impossible for other crates to manage it"
                )
            }
            Ok(())
        })
        .build()
}

mod sealed {
    use super::*;

    pub const UNINITIALIZED_ERR_MSG: &str = "The plugin is not initialized";

    pub struct PyFutureRunnerState<'a>(pub(super) State<'a, PyFutureRunner>);

    impl Deref for PyFutureRunnerState<'_> {
        type Target = PyFutureRunnerInner;

        fn deref(&self) -> &Self::Target {
            &self.0 .0
        }
    }

    pub struct PyCommandsState<'a>(pub(super) State<'a, PyCommands>);

    impl Deref for PyCommandsState<'_> {
        type Target = PyCommandsInner;

        fn deref(&self) -> &Self::Target {
            &self.0 .0
        }
    }

    pub trait SealedTrait<R> {}

    impl<R: Runtime, T: Manager<R>> SealedTrait<R> for T {}
}

use sealed::{PyCommandsState, PyFutureRunnerState, SealedTrait, UNINITIALIZED_ERR_MSG};

pub trait PyTauriExt<R: Runtime>: Manager<R> + SealedTrait<R> {
    fn try_pyfuture_runner(&self) -> Option<PyFutureRunnerState<'_>> {
        self.try_state::<PyFutureRunner>().map(PyFutureRunnerState)
    }

    /// The return type is equivalent to &[PyFutureRunnerInner].
    ///
    /// # Panic
    ///
    /// If the plugin is not initialized.
    fn pyfuture_runner(&self) -> PyFutureRunnerState<'_> {
        self.try_pyfuture_runner().expect(UNINITIALIZED_ERR_MSG)
    }

    fn try_pycommands(&self) -> Option<PyCommandsState<'_>> {
        self.try_state::<PyCommands>().map(PyCommandsState)
    }

    /// The return type is equivalent to &[PyCommandsInner].
    ///
    /// # Panic
    ///
    /// If the plugin is not initialized.
    fn pycommands(&self) -> PyCommandsState<'_> {
        self.try_pycommands().expect(UNINITIALIZED_ERR_MSG)
    }
}

impl<R: Runtime, T: Manager<R>> PyTauriExt<R> for T {}
