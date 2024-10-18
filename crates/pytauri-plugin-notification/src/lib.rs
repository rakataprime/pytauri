use pyo3::exceptions;
use pyo3::prelude::*;
use tauri_plugin_notification as plugin;
use tauri_plugin_notification::NotificationExt as _;
use tauri_plugin_pytauri::{pytauri, Runtime};

struct PluginError(plugin::Error);

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

#[pyclass]
#[non_exhaustive]
struct NotificationBuilder {
    _inner: Option<plugin::NotificationBuilder<Runtime>>,
}

impl NotificationBuilder {
    const fn new(inner: plugin::NotificationBuilder<Runtime>) -> Self {
        Self {
            _inner: Some(inner),
        }
    }

    /// Take the inner `NotificationBuilder` from `self_`,
    /// if `NotificationBuilder` is already consumed, return `PyRuntimeError`.

    // NOTE: `#[inline]` is important:
    // This function essentially acts as a macro to reduce repetitive code in `pymethods`.
    // We need to inline it to inform the compiler of our changes to `self._inner`,
    // so that subsequent operations on `self._inner` can be optimized.
    #[inline]
    fn _inner(&mut self) -> PyResult<plugin::NotificationBuilder<Runtime>> {
        self._inner.take().ok_or_else(|| {
            exceptions::PyRuntimeError::new_err("NotificationBuilder is already consumed")
        })
    }
}

#[pymethods]
impl NotificationBuilder {
    fn title(mut self_: PyRefMut<'_, Self>, title: String) -> PyResult<PyRefMut<'_, Self>> {
        let builder = self_._inner()?.title(title);
        let _ = self_._inner.insert(builder);
        Ok(self_)
    }

    fn body(mut self_: PyRefMut<'_, Self>, body: String) -> PyResult<PyRefMut<'_, Self>> {
        let builder = self_._inner()?.body(body);
        let _ = self_._inner.insert(builder);
        Ok(self_)
    }

    fn show(&mut self, py: Python<'_>) -> PyResult<()> {
        // TODO (perf): Do we really need `py.allow_threads` here?
        // I mean, I don't know how long `NotificationBuilder::show` will take,
        // maybe it's short enough?
        py.allow_threads(|| {
            self._inner()?.show().map_err(PluginError)?;

            Ok(())
        })
    }
}

// TODO(perf): we can't use struct `Notification` directly
//
// - Because `app_handle.notification()` returns `&Notification`,
//   however pyclass doesn't allow borrowing (i.g, ownership required).
//
//   > We should create a issue to `tauri` for `Notification.clone()`.
//
// - And, `Notification` is private, maybe is tauri's mistake.
//
//   > create a issue to `tauri` to make `Notification` public.
#[pyclass(frozen)]
#[non_exhaustive]
struct Notification {
    app_handle: Py<pytauri::AppHandle>,
}

#[pymethods]
impl Notification {
    fn builder(&self) -> NotificationBuilder {
        // NOTE: this function is simple enough,
        // so we don't need to use `py.allow_threads`

        let builder = self.app_handle.get().0.notification().builder();
        NotificationBuilder::new(builder)
    }
}

#[pyclass(frozen)]
#[non_exhaustive]
struct NotificationExt {
    app_handle: Py<pytauri::AppHandle>,
}

#[pymethods]
impl NotificationExt {
    #[new]
    const fn new(app_handle: Py<pytauri::AppHandle>) -> Self {
        Self { app_handle }
    }

    fn notification(&self, py: Python<'_>) -> Notification {
        let app_handle = self.app_handle.bind(py).clone().unbind();
        Notification { app_handle }
    }
}

#[pymodule(submodule)]
pub mod notification {
    #[pymodule_export]
    use crate::NotificationExt;

    #[pymodule_export]
    use crate::Notification;

    #[pymodule_export]
    use crate::NotificationBuilder;
}
