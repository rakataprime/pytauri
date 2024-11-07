mod commands;
mod pymod;

use tauri::plugin::{Builder, TauriPlugin};

pub type Runtime = tauri::Wry;
pub use pymod::pymodule_export;

pub mod pytauri {
    pub use crate::pymod::{App, AppHandle, Commands, Runner, RunEvent};
}

pub fn init() -> TauriPlugin<Runtime> {
    Builder::new(pymod::PYO3_MOD_NAME)
        .invoke_handler(tauri::generate_handler![commands::pyfunc])
        .build()
}
