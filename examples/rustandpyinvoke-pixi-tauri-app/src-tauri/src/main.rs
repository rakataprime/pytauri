// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::{convert::Infallible, env::var, error::Error, path::PathBuf};

use pyo3::wrap_pymodule;
use pytauri::standalone::{
    dunce::simplified, PythonInterpreterBuilder, PythonInterpreterEnv, PythonScript,
};
use tauri::{Builder, Manager as _};

use tauri_app_lib::{ext_mod, tauri_generate_context};

fn main() -> Result<Infallible, Box<dyn Error>> {
    let py_env = if cfg!(dev) {
        // `cfg(dev)` is set by `tauri-build` in `build.rs`, which means running with `tauri dev`,
        // see: <https://github.com/tauri-apps/tauri/pull/8937>.

        let venv_dir = var("VIRTUAL_ENV").map_err(|err| {
            format!(
                "The app is running in tauri dev mode, \
                please activate the python virtual environment first \
                or set the `VIRTUAL_ENV` environment variable: {err}",
            )
        })?;
        PythonInterpreterEnv::Venv(PathBuf::from(venv_dir).into())
    } else {
        // embedded Python, i.e., bundle mode with `tauri build`.

        // Actually, we don't use this app, we just use it to get the resource directory
        let sample_app = Builder::default()
            .build(tauri_generate_context())
            .map_err(|err| format!("failed to build sample app: {err}"))?;
        let resource_dir = sample_app
            .path()
            .resource_dir()
            .map_err(|err| format!("failed to get resource dir: {err}"))?;

        // 👉 Remove the UNC prefix `\\?\`, Python ecosystems don't like it.
        let resource_dir = simplified(&resource_dir).to_owned();

        // 👉 When bundled as a standalone App, we will put python in the resource directory
        PythonInterpreterEnv::Standalone(resource_dir.into())
    };

    // 👉 Equivalent to `python -m tauri_app`,
    // i.e, run the `src-tauri/python/tauri_app/__main__.py`
    let py_script = PythonScript::Module("tauri_app".into());

    // 👉 `ext_mod` is your extension module, we export it from memory,
    // so you don't need to compile it into a binary file (.pyd/.so).
    let builder =
        PythonInterpreterBuilder::new(py_env, py_script, |py| wrap_pymodule!(ext_mod)(py));
    let interpreter = builder.build()?;

    let exit_code = interpreter.run();

   // Continue with the Tauri builder setup
   println!("Setting up Tauri...");
   tauri::Builder::default()
       .plugin(tauri_plugin_dialog::init())
       .plugin(tauri_plugin_fs::init())
       .plugin(tauri_plugin_pytauri::init(py_invoke_handler))
       .setup(|app| {
           app.manage(interpreter);
           app.manage(load_state(&app.handle()));
           Ok(())
       })
       .invoke_handler(tauri::generate_handler![
           greet,
       ])
       .run(generate_context!())
       .expect("error while running tauri application");

    std::process::exit(exit_code);
}



// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::path::{Path, PathBuf};
use tauri::command;
use tauri::Manager;
use tauri::Runtime;
// use gix
// use std::process::Command;
mod clone; // Importing the clone module
use std::fs;
use tauri_app_lib::clone_repo_f;
use tauri_app_lib::test_f;

mod cuda;
mod docker;
mod intel;
mod prereqs;
mod py;
mod rocm;
mod simlink;
mod wsl;
// use ThumperRsun::write_config;
use elevated_command::Command as ElevatedCommand;
// use anyio;
use tauri::{AppHandle, State};

use regex::Regex;
use sys_info::{disk_info, mem_info, os_release, os_type}; // use std::process::Command;

use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::Read;
use std::io::Write;
use std::sync::Mutex;

use async_std::task;
use std::env;
use std::process::Command as StdCommand;

use pyo3::wrap_pymodule;
use pytauri::standalone::{
    dunce::simplified, PythonInterpreterBuilder, PythonInterpreterEnv, PythonScript,
};

use tauri::{Builder, Manager as _, Context};
use tauri_app_lib::ext_mod;
#[cfg(not(feature = "custom-protocol"))]
use tauri::generate_context;

use pyo3::Python;
use pyo3::types::PyAnyMethods;

#[command]
fn greet(name: &str) -> String {
    println!("Message from Rust:");
    format!("Hello, {}! You've been greeted from Rust!", name).into()
    // println!("Message from Rust: {}", name)
}

fn main() {
    // Enable verbose Python initialization for debugging
    std::env::set_var("PYTHONVERBOSE", "1");
    std::env::set_var("PYTHONDEBUG", "1");

    let current_dir = std::env::current_dir()
        .expect("Failed to get current directory");
    let pixi_env = current_dir.join(".pixi/envs/default");

    if !pixi_env.exists() {
        panic!("Pixi environment not found at {:?}. Please run 'pixi install' first.", pixi_env);
    }

    // Verify Python binary exists and print its version
    let python_binary = pixi_env.join("bin/python3");
    if !python_binary.exists() {
        panic!("Python binary not found at {:?}", python_binary);
    }

    // Get detailed Python version info
    let version_output = std::process::Command::new(&python_binary)
        .arg("-c")
        .arg(r#"
import sys
print(f'{sys.version_info.major}.{sys.version_info.minor}')
print(sys.prefix)
print(sys.exec_prefix)
print(sys.base_prefix)
"#)
        .output()
        .expect("Failed to get Python version");
    
    // Create a longer-lived string from the output
    let version_string = String::from_utf8_lossy(&version_output.stdout).into_owned();
    let version_info: Vec<String> = version_string.trim()
        .split('\n')
        .map(|s| s.to_string())
        .collect();
    
    let python_version = &version_info[0];
    println!("Detected Python version: {}", python_version);

    // Get absolute path for the Python environment
    let pythonhome = pixi_env.canonicalize()
        .expect("Failed to get absolute path for PYTHONHOME");

    println!("Setting up Python environment at: {:?}", pythonhome);

    // Get the path to our Python module
    let module_path = current_dir.join("python");
    if !module_path.exists() {
        std::fs::create_dir_all(&module_path).expect("Failed to create python module directory");
    }
    
    let module_path = module_path.canonicalize()
        .expect("Failed to get absolute path for module directory");
    println!("Python module path: {:?}", module_path);

    // Set PYTHONPATH to include our module
    std::env::set_var("PYTHONPATH", module_path.to_str().unwrap());

    // Initialize Python with minimal configuration
    let py_env = PythonInterpreterEnv::Standalone(pythonhome.clone().into());

    // Set only essential environment variables
    std::env::set_var("PYTHONVERBOSE", "1");
    std::env::set_var("PYTHONDEBUG", "1");
    std::env::set_var("PYTHONDONTWRITEBYTECODE", "1");

    println!("Attempting to build Python interpreter...");
    // Create the Python interpreter with minimal configuration
    let interpreter = PythonInterpreterBuilder::new(
        py_env,
        PythonScript::Module("ThumperRun".into()),
        |py| wrap_pymodule!(tauri_app_lib::ext_mod)(py)
    )
    // .with_async_backend(anyio::tokio::backend::TokioBackend)
    .build()
    .expect("Failed to build Python interpreter");

    println!("Successfully built Python interpreter");

    // Get the Python invoke handler using pyo3
    println!("Getting Python invoke handler...");
    let py_invoke_handler = Python::with_gil(|py| {
        let thumper_run = py.import("ThumperRun")
            .expect("Failed to import ThumperRun module");
        
        // Initialize the module
        thumper_run.call_method0("init")
            .expect("Failed to initialize ThumperRun module");
        
        // Get the invoke handler
        thumper_run.call_method0("get_invoke_handler")
            .expect("Failed to get invoke handler")
            .into()
    });
    println!("Successfully got Python invoke handler");

    // Continue with the Tauri builder setup
    println!("Setting up Tauri...");
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_pytauri::init(py_invoke_handler))
        .setup(|app| {
            app.manage(interpreter);
            app.manage(load_state(&app.handle()));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            greet,
        ])
        .run(generate_context!())
        .expect("error while running tauri application");
}
