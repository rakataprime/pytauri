use std::fs;
use std::io;
use std::path::Path;

use pyo3::ffi as pyffi;
use pyo3::ffi::c_str;
use pyo3::prelude::*;
pub use pyo3::prepare_freethreaded_python;
use pyo3::types::PyModule;

use crate::pyembed::{utils, NewInterpreterError, NewInterpreterResult};

#[expect(dead_code)] // TODO: remove this attribute
enum PyConfigProfile {
    Python,
    Isolated,
}

struct PyConfig(pyffi::PyConfig);

// ref: <https://github.com/indygreg/PyOxidizer/blob/1ceca8664c71f39e849ce4873e00d821504b32bd/pyembed/src/interpreter_config.rs#L252-L619>
impl PyConfig {
    fn new(profile: PyConfigProfile) -> Self {
        let mut config: pyffi::PyConfig = unsafe { std::mem::zeroed() };

        unsafe {
            match profile {
                PyConfigProfile::Isolated => pyffi::PyConfig_InitIsolatedConfig(&mut config),
                PyConfigProfile::Python => pyffi::PyConfig_InitPythonConfig(&mut config),
            }
        }

        Self(config)
    }

    fn set_prefix(&mut self, prefix: &Path) -> NewInterpreterResult<()> {
        utils::set_config_string_from_path(&self.0, &self.0.prefix, prefix, "setting prefix")
    }

    fn set_base_prefix(&mut self, base_prefix: &Path) -> NewInterpreterResult<()> {
        utils::set_config_string_from_path(
            &self.0,
            &self.0.base_prefix,
            base_prefix,
            "setting base_prefix",
        )
    }

    fn set_exec_prefix(&mut self, exec_prefix: &Path) -> NewInterpreterResult<()> {
        utils::set_config_string_from_path(
            &self.0,
            &self.0.exec_prefix,
            exec_prefix,
            "setting exec_prefix",
        )
    }

    fn set_base_exec_prefix(&mut self, base_exec_prefix: &Path) -> NewInterpreterResult<()> {
        utils::set_config_string_from_path(
            &self.0,
            &self.0.base_exec_prefix,
            base_exec_prefix,
            "setting base_exec_prefix",
        )
    }

    // ref: <https://github.com/indygreg/PyOxidizer/blob/1ceca8664c71f39e849ce4873e00d821504b32bd/pyembed/src/interpreter.rs#L130-L255>
    fn init(self) -> NewInterpreterResult<()> {
        let status = unsafe { pyffi::Py_InitializeFromConfig(&self.0) };
        if unsafe { pyffi::PyStatus_Exception(status) } != 0 {
            return Err(NewInterpreterError::new_from_pystatus(
                &status,
                "initializing Python core",
            ));
        }
        // The GIL is held.
        debug_assert_eq!(unsafe { pyffi::PyGILState_Check() }, 1);

        // We release the GIL so we can have pyo3's GIL handling take over from
        // an "empty" state. This mirrors what pyo3's prepare_freethreaded_python() does.
        unsafe {
            pyffi::PyEval_SaveThread();
        }
        Ok(())
    }
}

impl Drop for PyConfig {
    fn drop(&mut self) {
        unsafe {
            pyffi::PyConfig_Clear(&mut self.0);
        }
    }
}

fn get_pythonhome_from_pyvenv_cfg(venv_path: &Path) -> Option<String> {
    let pyvenv_cfg = venv_path.join("pyvenv.cfg");
    let cfg = fs::read_to_string(pyvenv_cfg).ok()?;
    // cfg example:
    //
    // ```
    // home = C:\Users\UserName\AppData\Local\Programs\Python\Python310
    // implementation = CPython
    // uv = 0.5.4
    // version_info = 3.10.8
    // include-system-site-packages = false
    // prompt = pytauri-workspace
    // ```
    let pythonhome = cfg.lines().find(|line| line.starts_with("home"))?;
    let pythonhome = pythonhome.split('=').nth(1)?.trim();
    Some(pythonhome.to_owned())
}

/// see:
/// - <https://github.com/PyO3/pyo3/issues/3589>
/// - <https://github.com/PyO3/pyo3/issues/1896>
///
/// pyo3 currently cannot automatically detect the virtual environment and configure pyconfig, so we need this function.
///
/// # Panics
///
/// This function will panic if it cannot find the python home from `pyvenv.cfg`.
pub fn prepare_freethreaded_python_venv(venv_path: impl AsRef<Path>) -> NewInterpreterResult<()> {
    let venv_path = venv_path.as_ref();

    let pythonhome = get_pythonhome_from_pyvenv_cfg(venv_path)
        .expect("Cannot find python home from `pyvenv.cfg`, please check your python venv");
    let pythonhome = Path::new(&pythonhome);

    let mut config = PyConfig::new(PyConfigProfile::Python);
    config.set_prefix(venv_path)?;
    config.set_exec_prefix(venv_path)?;
    config.set_base_prefix(pythonhome)?;
    config.set_base_exec_prefix(pythonhome)?;
    config.init()?;

    // At this point, the Python interpreter has already been initialized, so this is a no-op,
    // but we still need to call it to let pyo3 know that we have initialized the Python interpreter.
    prepare_freethreaded_python();
    Ok(())
}

/// Insert the `pytauri` extension module into `sys.modules`, otherwise your Python code cannot import `pytauri`.
pub fn append_ext_mod(ext_mod: Bound<PyModule>) -> PyResult<()> {
    let py = ext_mod.py();

    let _append_ext_mod = PyModule::from_code(
        py,
        c_str!(include_str!("_append_ext_mod.py")),
        c_str!("_append_ext_mod.py"),
        c_str!("_append_ext_mod"),
    )?;

    _append_ext_mod
        .getattr("append_ext_mod")?
        .call1((ext_mod,))?;

    Ok(())
}

pub fn write_py_err_to_file(
    py: Python<'_>,
    py_err: &PyErr,
    file_path: impl AsRef<Path>,
) -> io::Result<()> {
    if let Some(tb) = py_err.traceback(py) {
        if let Ok(tb) = tb.format() {
            return fs::write(file_path, tb);
        }
    }
    fs::write(file_path, format!("{:?}", py_err))
}
