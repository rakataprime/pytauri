use std::env;
use std::path::PathBuf;

const COMMANDS: &[&str] = &["pyfunc"];

fn main() {
    // for `#[cfg(not(Py_GIL_DISABLED))]`,
    // see <https://pyo3.rs/v0.23.2/building-and-distribution/multiple-python-versions.html#using-pyo3-build-config>
    pyo3_build_config::use_pyo3_cfgs();

    // <https://doc.rust-lang.org/cargo/reference/environment-variables.html#environment-variables-cargo-sets-for-build-scripts>
    let is_debug = env::var("DEBUG").unwrap() == "true";

    let global_api_script_dir = PathBuf::from("guest-js/dist");
    let mut global_api_script_path = global_api_script_dir.join("api-iife");
    if is_debug {
        global_api_script_path.set_extension("dev.js");
    } else {
        global_api_script_path.set_extension("prod.js");
    }

    tauri_plugin::Builder::new(COMMANDS)
        .global_api_script_path(global_api_script_path)
        .build();
}
