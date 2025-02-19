// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::{env::{var, set_var}, path::PathBuf};
use env_logger;
use log::{debug, error, info, warn};

#[allow(deprecated)]
use pyo3::{wrap_pymodule, Python, types::PyAnyMethods, PyObject, IntoPy};
use pytauri::standalone::{
    dunce::simplified, PythonInterpreterBuilder, PythonInterpreterEnv, PythonScript,
};
use tauri::{Builder, Manager as _, generate_context};

use tauri_app_lib::{ext_mod, tauri_generate_context};

// Simple state management
#[derive(Default)]
struct AppState {
    // Add any state fields you need
}

fn load_state(_app_handle: &tauri::AppHandle) -> AppState {
    AppState::default()
}

fn main() {
    // Initialize logging
    env_logger::init();
    info!("Starting application...");

    // Set up Python path to include our module
    let current_dir = std::env::current_dir().expect("Failed to get current directory");
    let python_path = current_dir.join("python");
    
    debug!("Setting PYTHONPATH to include: {}", python_path.display());
    if let Ok(existing_path) = var("PYTHONPATH") {
        set_var("PYTHONPATH", format!("{}:{}", python_path.display(), existing_path));
    } else {
        set_var("PYTHONPATH", python_path);
    }

    let py_env = if cfg!(dev) {
        debug!("Running in dev mode");
        let venv_dir = var("VIRTUAL_ENV").expect(
            "The app is running in tauri dev mode, \
            please activate the python virtual environment first \
            or set the `VIRTUAL_ENV` environment variable",
        );
        info!("Using virtual environment at: {}", venv_dir);
        PythonInterpreterEnv::Venv(PathBuf::from(venv_dir).into())
    } else {
        debug!("Running in production mode");
        let context = tauri_generate_context();
        let app = Builder::default()
            .build(context)
            .expect("Error building Tauri application");
            
        let resource_dir = app
            .path()
            .resource_dir()
            .expect("Failed to get resource dir");

        debug!("Resource directory: {}", resource_dir.display());
        let resource_dir = simplified(&resource_dir).to_owned();
        PythonInterpreterEnv::Standalone(resource_dir.into())
    };

    info!("Setting up Python interpreter...");
    let py_script = PythonScript::Module("tauri_app".into());
    let pybuilder = PythonInterpreterBuilder::new(py_env, py_script, |py| {
        debug!("Initializing Python extension module");
        wrap_pymodule!(ext_mod)(py)
    });
    
    let interpreter = match pybuilder.build() {
        Ok(i) => {
            info!("Python interpreter built successfully");
            i
        },
        Err(e) => {
            error!("Failed to build Python interpreter: {}", e);
            panic!("Failed to build Python interpreter: {}", e);
        }
    };

    // Get the Python invoke handler using the GIL
    info!("Setting up Python invoke handler...");
    let py_invoke_handler: PyObject = Python::with_gil(|py| {
        debug!("Acquiring Python GIL");
        match py.import("tauri_app") {
            Ok(tauri_app) => {
                debug!("Successfully imported tauri_app module");
                match tauri_app.getattr("commands") {
                    Ok(commands) => {
                        debug!("Successfully got commands object");
                        commands.into_py(py)
                    },
                    Err(e) => {
                        error!("Failed to get commands object: {}", e);
                        panic!("Failed to get commands object: {}", e);
                    }
                }
            },
            Err(e) => {
                error!("Failed to import tauri_app module: {}", e);
                panic!("Failed to import tauri_app module: {}", e);
            }
        }
    });

    info!("Initializing Tauri application...");
    // Traditional Tauri App Launch with plugin/init   
    Builder::default()
           .plugin(tauri_plugin_pytauri::init(py_invoke_handler))
           .setup(|app| {
               debug!("Running Tauri setup");
               app.manage(interpreter);
               app.manage(load_state(&app.handle()));
               Ok(())
           })
           .invoke_handler(tauri::generate_handler![tauri_app_lib::greet_rust])
           .run(generate_context!())
           .expect("error while running tauri application");
}

