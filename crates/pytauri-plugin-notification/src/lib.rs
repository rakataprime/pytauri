mod ext_mod_impl;

use pyo3::prelude::*;

#[pymodule(submodule, gil_used = false)]
pub mod notification {
    use super::*;

    #[pymodule_export]
    pub use ext_mod_impl::{NotificationBuilder, NotificationBuilderArgs, NotificationExt};

    pub use ext_mod_impl::ImplNotificationExt;
}

pub use notification as ext_mod;
