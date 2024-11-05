use std::cell::{Ref, RefCell, RefMut};
use std::thread_local;

use dashmap::DashMap;
pub use pyfuture::{future::PyFuture, runner::Runner};
use pyo3::exceptions::{PyRuntimeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyCFunction, PyDict, PyModule, PyTuple};
use tauri::{Context, Manager as _};

use crate::Runtime;

pub(crate) const PYO3_MOD_NAME: &str = "pytauri";

// TODO: newtype for these to prevent conflicts with other plugins
pub type FutureRunner = Py<Runner>;
pub type PyCommands = Py<Commands>;

trait PyMatchMethods {
    type Output;
    fn r#match(&self) -> Self::Output;
}

macro_rules! impl_py_match_methods {
    ($cls:ty, $ret:ty) => {
        #[pymethods]
        impl $cls {
            fn r#match(&self) -> $ret {
                <Self as $crate::pymod::PyMatchMethods>::r#match(self)
            }
        }
    };
}

#[pyclass(frozen)]
#[non_exhaustive]
enum RunEventEnum {
    Exit(),
    #[non_exhaustive]
    ExitRequested {
        code: Option<i32>,
        // TODO, XXX, FIXME: `ExitRequestApi` is a private type in `tauri`,
        // we need create a issue to `tauri`, or we cant implement this.
        // api: ExitRequestApi,
    },
    #[non_exhaustive]
    WindowEvent {
        label: String,
        // TODO:
        // event: WindowEvent,
    },
    #[non_exhaustive]
    WebviewEvent {
        label: String,
        // TODO:
        // event: WebviewEvent,
    },
    Ready(),
    Resumed(),
    MainEventsCleared(),
    MenuEvent(/* TODO: tauri::menu::MenuEvent */),
    // TODO:
    // TrayIconEvent(tauri::tray::TrayIconEvent),
}

#[pyclass(frozen)]
#[non_exhaustive]
pub struct RunEvent(pub tauri::RunEvent);

impl PyMatchMethods for RunEvent {
    type Output = RunEventEnum;

    fn r#match(&self) -> Self::Output {
        match &self.0 {
            tauri::RunEvent::Exit => RunEventEnum::Exit(),
            tauri::RunEvent::ExitRequested {
                code, /* TODO */ ..
            } => RunEventEnum::ExitRequested { code: *code },
            tauri::RunEvent::WindowEvent {
                label, /* TODO */ ..
            } => RunEventEnum::WindowEvent {
                label: label.to_owned(),
            },
            tauri::RunEvent::WebviewEvent {
                label, /* TODO */ ..
            } => RunEventEnum::WebviewEvent {
                label: label.to_owned(),
            },
            tauri::RunEvent::Ready => RunEventEnum::Ready(),
            tauri::RunEvent::Resumed => RunEventEnum::Resumed(),
            tauri::RunEvent::MainEventsCleared => RunEventEnum::MainEventsCleared(),
            tauri::RunEvent::MenuEvent(/* TODO */ _) => RunEventEnum::MenuEvent(),
            // TODO: tauri::RunEvent::TrayIconEvent,
            event => unimplemented!("Unimplemented RunEvent: {event:?}"),
        }
    }
}

impl_py_match_methods!(RunEvent, RunEventEnum);

#[pyclass(frozen)]
#[non_exhaustive]
pub struct AppHandle(pub tauri::AppHandle<Runtime>);

#[pyclass(frozen)]
pub struct App;

impl App {
    // `Send` is required for `pyclass`, `tauri::App` is `!Send`,
    // so we have to make it thread local singleton.
    thread_local! {
        static APP_INST: RefCell<Option<tauri::App>> = const { RefCell::new(None) };
    }

    pub(self) fn try_build(
        app_inst: tauri::App,
        future_runner: FutureRunner,
        py_commands: PyCommands,
    ) -> Result<Self, tauri::App> {
        Self::APP_INST.with(|app_inst_cell| {
            let mut app_inst_cell = match app_inst_cell.try_borrow_mut() {
                Ok(app_inst_cell) => app_inst_cell,
                // If Err, it means this thread has already mutably borrowed and used the app instance
                // i.e., the app has already been created.
                Err(_) => return Err(app_inst),
            };
            // The app instance has already been created.
            if app_inst_cell.is_some() {
                return Err(app_inst);
            }

            // if false, there has already state set for the app instance.
            if !app_inst.manage(future_runner) {
                panic!("DO NOT set `FutureRunner` for App manually!");
            }
            if !app_inst.manage(py_commands) {
                panic!("DO NOT set `Commands` for App manually!");
            }

            // Ok, we create the new app instance.
            *app_inst_cell = Some(app_inst);
            Ok(Self)
        })
    }

    fn try_borrow_mut_app_cell(
        app_inst_cell: &RefCell<Option<tauri::App>>,
    ) -> PyResult<RefMut<'_, Option<tauri::App>>> {
        app_inst_cell
            .try_borrow_mut()
            .map_err(|_| PyRuntimeError::new_err("The app is currently borrowed"))
    }

    fn try_borrow_app_cell(
        app_inst_cell: &RefCell<Option<tauri::App>>,
    ) -> PyResult<Ref<'_, Option<tauri::App>>> {
        app_inst_cell
            .try_borrow()
            .map_err(|_| PyRuntimeError::new_err("The app is currently mutably borrowed"))
    }

    fn map_none_to_py_err<T>(opt: Option<T>) -> PyResult<T> {
        opt.ok_or_else(|| {
            PyRuntimeError::new_err(
                "The app has already been costumed or you call this method in the wrong thread",
            )
        })
    }

    // TODO, PERF: maybe we can make `callback` optional
    // and don't call it so that we don't have to require GIL
    fn py_cb_to_rs_cb(
        callback: PyObject,
    ) -> impl FnMut(&tauri::AppHandle<Runtime>, tauri::RunEvent) {
        move |app_handle, run_event| {
            let py_app_handle = AppHandle(app_handle.to_owned());
            let py_run_event = RunEvent(run_event);

            Python::with_gil(|py| {
                let result = callback.call1(py, (py_app_handle, py_run_event));
                if let Err(e) = result {
                    // TODO, XXX: maybe we should use `write_unraisable_bound` and panic here?
                    e.restore(py);
                }
            })
        }
    }
}

// TODO, FIXME, XXX: drop `APP_INST` in `__del__`,
// See also: <https://github.com/PyO3/pyo3/issues/2479>,
// maybe we need subclass it in python for `__del__` method.
#[pymethods]
impl App {
    fn run(&self, py: Python<'_>, callback: PyObject) -> PyResult<()> {
        py.allow_threads(|| {
            Self::APP_INST.with(|app_inst_cell| {
                let mut app_inst_ref_mut = Self::try_borrow_mut_app_cell(app_inst_cell)?;
                let app = Self::map_none_to_py_err(app_inst_ref_mut.take())?;
                app.run(Self::py_cb_to_rs_cb(callback));
                Ok(())
            })
        })
    }

    fn run_iteration(&self, py: Python<'_>, callback: PyObject) -> PyResult<()> {
        py.allow_threads(|| {
            Self::APP_INST.with(|app_inst_cell| {
                let mut app_inst_ref_mut = Self::try_borrow_mut_app_cell(app_inst_cell)?;
                let app = Self::map_none_to_py_err(app_inst_ref_mut.as_mut())?;
                app.run_iteration(Self::py_cb_to_rs_cb(callback));
                Ok(())
            })
        })
    }

    fn cleanup_before_exit(&self, py: Python<'_>) -> PyResult<()> {
        py.allow_threads(|| {
            Self::APP_INST.with(|app_inst_cell| {
                let app_inst_ref = Self::try_borrow_app_cell(app_inst_cell)?;
                let app = Self::map_none_to_py_err(app_inst_ref.as_ref())?;
                app.cleanup_before_exit();
                Ok(())
            })
        })
    }
}

#[pyclass(subclass)]
pub struct Commands {
    // TODO: use PyDict instead
    pub(crate) handlers: DashMap<String, PyObject>,
}

#[pymethods]
impl Commands {
    #[new]
    fn new() -> Self {
        Self {
            handlers: DashMap::new(),
        }
    }

    /// Register a async python function to be called from Rust.
    /// `py_func`: Callable[..., Awaitable[bytes]], see implementation for `...`
    fn invoke_handler(&mut self, func_name: String, py_func: PyObject) -> PyResult<()> {
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
        let future_runner = args.get_item(0)?.downcast_into::<Runner>()?.unbind();
        let py_commands = args.get_item(1)?.downcast_into::<Commands>()?.unbind();

        let tauri_app = app_builder(kwargs)?
            .build(context_builder()?)
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to build tauri app: {:?}", e)))?;

        App::try_build(tauri_app, future_runner, py_commands).map_err(|_| {
            PyRuntimeError::new_err("An app instance has already been created in this thread")
        })
    };
    let build_app = PyCFunction::new_closure_bound(
        py,
        Some(c"build_app"),
        Some(c"build tauri app"),
        build_app_closure,
    )?;

    let self_module = PyModule::new_bound(py, PYO3_MOD_NAME)?;
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
