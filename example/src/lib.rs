use pyo3::prelude::*;

fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_pytauri::init())
        .plugin(tauri_plugin_notification::init())
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[pymodule]
#[pyo3(name = "_ext_mod")]
mod _ext_mod {
    use super::*;

    /// Run the tauri application.
    #[pyfunction]
    fn run() {
        super::run()
    }

    #[pymodule_export]
    use tauri_plugin_pytauri::pytauri;

    #[pymodule_export]
    use pytauri_plugin_notification::notification;
}
