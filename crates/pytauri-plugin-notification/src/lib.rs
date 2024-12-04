use std::error::Error;
use std::fmt::{Debug, Display};

use pyo3::prelude::*;
use pyo3_utils::{PyWrapper, PyWrapperT2};
use pytauri_core::tauri_runtime::Runtime;
use pytauri_core::AppHandle;
use tauri_plugin_notification as plugin;
use tauri_plugin_notification::NotificationExt as _;

#[derive(Debug)]
struct PluginError(plugin::Error);

impl Display for PluginError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        <Self as Debug>::fmt(self, f)
    }
}

impl Error for PluginError {}

impl From<PluginError> for PyErr {
    fn from(value: PluginError) -> Self {
        match value.0 {
            plugin::Error::Io(e) => e.into(),
        }
    }
}

impl From<plugin::Error> for PluginError {
    fn from(value: plugin::Error) -> Self {
        Self(value)
    }
}

// We use a `newtype` instead of directly implementing methods like `show(title=...)`,
// This is to facilitate the addition of new methods in the future,
// such as `hide(NotificationBuilderArgs)` instead of repeatedly declaring `hide(title=...)`
#[pyclass(frozen)]
#[non_exhaustive]
#[derive(Clone, Debug)]
pub struct NotificationBuilderArgs {
    pub title: Option<String>,
    pub body: Option<String>,
}

#[pymethods]
impl NotificationBuilderArgs {
    #[new]
    #[pyo3(signature = (*, title = None, body = None))]
    fn new(title: Option<String>, body: Option<String>) -> Self {
        Self { title, body }
    }
}

impl NotificationBuilderArgs {
    fn call_show(self, mut builder: plugin::NotificationBuilder<Runtime>) -> plugin::Result<()> {
        let Self { title, body } = self;
        if let Some(v) = title {
            builder = builder.title(v)
        }
        if let Some(v) = body {
            builder = builder.body(v)
        }
        builder.show()
    }
}

#[pyclass(frozen)]
#[non_exhaustive]
pub struct NotificationBuilder(pub PyWrapper<PyWrapperT2<plugin::NotificationBuilder<Runtime>>>);

impl NotificationBuilder {
    fn new(builder: plugin::NotificationBuilder<Runtime>) -> Self {
        Self(PyWrapper::new2(builder))
    }
}

#[pymethods]
impl NotificationBuilder {
    fn show(&self, py: Python<'_>, args: NotificationBuilderArgs) -> PyResult<()> {
        // TODO (perf): Do we really need `py.allow_threads` here?
        // I mean, I don't know how long `NotificationBuilder::show` will take,
        // maybe it's short enough?
        py.allow_threads(|| {
            let builder = self.0.try_take_inner()??;
            args.call_show(builder)
                .map_err(Into::<PluginError>::into)
                .map_err(Into::<PyErr>::into)
        })
    }
}

// TODO, FIXME, PEFR: we can't use struct `Notification` directly
//
// - Because `app_handle.notification()` returns `&Notification`,
//   however pyclass doesn't allow borrowing (i.g, ownership required).
//
//   > - We should create a issue to `tauri` for `Notification.clone()`
//   > - Or we can use [pyo3_utils::{Deref, DerefMut}] convention
//
// - And, `Notification` is private, maybe is tauri's mistake.
//
//   > create a issue to `tauri` to make `Notification` public.
#[pyclass(frozen)]
#[non_exhaustive]
pub struct Notification {
    app_handle: Py<AppHandle>,
}

impl Notification {
    const fn new(app_handle: Py<AppHandle>) -> Self {
        Self { app_handle }
    }
}

#[pymethods]
impl Notification {
    fn builder(&self) -> NotificationBuilder {
        // NOTE: this function is simple enough,
        // so we don't need to use `py.allow_threads`
        let builder = self.app_handle.get().0.inner_ref().notification().builder();
        NotificationBuilder::new(builder)
    }
}

#[pyclass(frozen)]
#[non_exhaustive]
pub struct NotificationExt {
    app_handle: Py<AppHandle>,
}

#[pymethods]
impl NotificationExt {
    #[new]
    const fn new(app_handle: Py<AppHandle>) -> Self {
        Self { app_handle }
    }

    fn notification(&self, py: Python<'_>) -> Notification {
        let app_handle = self.app_handle.bind(py).clone().unbind();
        Notification::new(app_handle)
    }
}

#[pymodule(submodule, gil_used = false)]
pub mod notification {
    #[pymodule_export]
    pub use crate::{Notification, NotificationBuilder, NotificationBuilderArgs, NotificationExt};
}
