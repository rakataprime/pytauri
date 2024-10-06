use pyo3::prelude::*;
use tauri_plugin_pytauri as pytauri;

fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(pytauri::init())
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

    #[pymodule_init]
    fn init(m: &Bound<'_, PyModule>) -> PyResult<()> {
        pytauri::register_pyo3_module(m)?;
        Ok(())
    }
}
