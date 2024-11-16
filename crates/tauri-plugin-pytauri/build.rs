use std::env;
use std::path::PathBuf;

const COMMANDS: &[&str] = &["pyfunc"];

fn main() {
    // <https://doc.rust-lang.org/cargo/reference/environment-variables.html#environment-variables-cargo-sets-for-build-scripts>
    let is_debug = env::var("DEBUG").unwrap() == "true";

    let global_api_script_dir = PathBuf::from("dist");
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
