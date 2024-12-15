mod ext_mod_impl;
pub mod tauri_runtime;
pub mod utils;

use pyo3::prelude::*;

#[pymodule(submodule, gil_used = false, name = "pytauri")]
pub mod ext_mod {
    use super::*;

    #[pymodule_export]
    pub use ext_mod_impl::{App, AppHandle, Context, RunEvent, RunEventEnum};

    pub use ext_mod_impl::{PyAppHandleExt, PyAppHandleStateError, PyAppHandleStateResult};

    #[pymodule]
    pub mod ipc {
        use super::*;

        #[pymodule_export]
        pub use ext_mod_impl::ipc::{Invoke, InvokeResolver};
    }
}
