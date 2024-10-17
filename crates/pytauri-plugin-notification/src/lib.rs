use pyo3::prelude::*;
use tauri_plugin_notification as plugin;
use tauri_plugin_notification::NotificationExt as _;
use tauri_plugin_pytauri::pytauri;

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
    _app_handle: Py<pytauri::AppHandle>,
    title: Option<String>,
    body: Option<String>,
}

#[pymethods]
impl NotificationBuilder {
    fn title(mut self_: PyRefMut<'_, Self>, title: String) -> PyRefMut<'_, Self> {
        self_.title = Some(title);
        self_
    }

    fn body(mut self_: PyRefMut<'_, Self>, body: String) -> PyRefMut<'_, Self> {
        self_.body = Some(body);
        self_
    }

    fn show(&self, py: Python<'_>) -> PyResult<()> {
        py.allow_threads(|| {
            let app_handle = &self._app_handle.get().0;
            let mut builder = app_handle.notification().builder();

            if let Some(title) = &self.title {
                builder = builder.title(title);
            }
            if let Some(body) = &self.body {
                builder = builder.body(body);
            }

            builder.show().map_err(PluginError)?;
            Ok(())
        })
    }
}

#[pyclass(frozen)]
#[non_exhaustive]
struct Notification {
    app_handle: Py<pytauri::AppHandle>,
}

#[pymethods]
impl Notification {
    fn builder(&self, py: Python<'_>) -> NotificationBuilder {
        let _app_handle = self.app_handle.bind(py).clone().unbind();
        NotificationBuilder {
            _app_handle,
            title: None,
            body: None,
        }
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
