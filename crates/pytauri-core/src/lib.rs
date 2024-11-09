pub mod tauri_runtime;

use std::cell::{Ref, RefCell, RefMut};
use std::thread_local;

use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::*;

use crate::tauri_runtime::Runtime;

trait PyMatchMethods {
    type Output;
    fn r#match(&self) -> Self::Output;
}

macro_rules! impl_py_match_methods {
    ($cls:ty, $ret:ty) => {
        #[pymethods]
        impl $cls {
            fn r#match(&self) -> $ret {
                <Self as $crate::PyMatchMethods>::r#match(self)
            }
        }
    };
}

#[pyclass(frozen)]
#[non_exhaustive]
pub enum RunEventEnum {
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

impl AppHandle {
    pub const fn new(app_handle: tauri::AppHandle<Runtime>) -> Self {
        Self(app_handle)
    }
}

#[pyclass(frozen)]
/// `#[non_exhaustive]` just make it private so that other crates can only create it by [App::try_build]
#[non_exhaustive]
pub struct App;

/// NOTE: all of rust api is private, you can only use python api.
impl App {
    // `Send` is required for `pyclass`, `tauri::App` is `!Send`,
    // so we have to make it thread local singleton.
    thread_local! {
        // NOTE: this static var must be private so that other crates can't not create it.
        static APP_INST: RefCell<Option<tauri::App>> = const { RefCell::new(None) };
    }

    pub fn try_build(app_inst: tauri::App) -> Result<Self, tauri::App> {
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

            // Ok, we create the new app instance.
            *app_inst_cell = Some(app_inst);
            Ok(Self)
        })
    }

    pub fn try_borrow_mut_app_cell(
        app_inst_cell: &RefCell<Option<tauri::App>>,
    ) -> PyResult<RefMut<'_, Option<tauri::App>>> {
        app_inst_cell
            .try_borrow_mut()
            .map_err(|_| PyRuntimeError::new_err("The app is currently borrowed"))
    }

    pub fn try_borrow_app_cell(
        app_inst_cell: &RefCell<Option<tauri::App>>,
    ) -> PyResult<Ref<'_, Option<tauri::App>>> {
        app_inst_cell
            .try_borrow()
            .map_err(|_| PyRuntimeError::new_err("The app is currently mutably borrowed"))
    }

    pub fn map_none_to_py_err<T>(opt: Option<T>) -> PyResult<T> {
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
