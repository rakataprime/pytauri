# Using pytauri

!!! note
    The dependency versions specified in the following tutorial are the versions at the time of writing. There may be newer versions available when you use it.

## Create venv

Create a virtual environment using `uv`:

```bash
uv venv --python-preference only-system
```

!!! warning
    `--python-preference only-system` is necessary. Using `uv`'s managed Python may result in not finding dynamic libraries.

activate the virtual environment:

=== "bash"
    ```bash
    source .venv/bin/activate
    ```

=== "powershell"
    ```powershell
    .venv\Scripts\Activate.ps1
    ```

## Init pyproject

Create the `src-tauri/python/tauri_app` folder to store Python code, and add the following file:

ref: <https://packaging.python.org/en/latest/guides/writing-pyproject-toml/>

```toml title="src-tauri/pyproject.toml" hl_lines="8 9"
[project]
name = "tauri-app"  # (1)!
version = "0.1.0"
description = "Add your description here"
requires-python = ">=3.9"
dependencies = ["pytauri == 0.1.*"]  # (2)!

[project.entry-points.pytauri]
ext_mod = "tauri_app.ext_mod"

[build-system]
requires = ["setuptools>=61"]
build-backend = "setuptools.build_meta"

[tool.setuptools.packages]
find = { where = ["python"] }  # (3)!
```

1. your python package name.
2. This is the version at the time of writing this tutorial. There may be a newer version of pytauri available when you use it.
3. the folder where your python code is stored, i.e., `src-tauri/python`.

!!! tip
    Note the highlighted `project.entry-points`. We will explain its specific meaning when building the Wheel. For now, let's continue with the tutorial.

## Install your project

Use `uv` to install your Python package in [editable mode](https://setuptools.pypa.io/en/latest/userguide/development_mode.html):

```bash
uv pip install -e src-tauri
```

Add following code:

```python title="src-tauri/python/tauri_app/__init__.py"
"""The tauri-app."""

from pytauri import (
    BuilderArgs,
    builder_factory,
    context_factory,
)


def main() -> None:
    """Run the tauri-app."""
    app = builder_factory().build(
        BuilderArgs(
            context=context_factory(),
        )
    )
    app.run()
```

```python title="src-tauri/python/tauri_app/__main__.py"
"""The main entry point for the Tauri app."""

from tauri_app import main

main()
```

## Run pytauri from rust

Add following dependencies to `Cargo.toml`:

ref: <https://doc.rust-lang.org/cargo/reference/cargo-targets.html#binaries>

```toml title="src-tauri/Cargo.toml"
# ...

[[bin]]
# the same as the package name
name = "tauri-app"
path = "src/main.rs"
required-features = ["pytauri/standalone"]

[dependencies]
# ...
pyo3 = { version = "0.23" }
pytauri = { version = "0.1.0-beta.0" }
```

Also, enable the `pytauri/standalone` feature:

```json title="src-tauri/tauri.conf.json"
{
    "build": {
        "features": ["pytauri/standalone"]
    }
}
```

!!! warning
    If you do not enable `required-features` in `tauri-cli`, [cargo will silently skip building your `main.rs` executable file](https://github.com/rust-lang/cargo/issues/4663).

Change following rust code:

```rust title="src-tauri/src/lib.rs"
use pyo3::prelude::*;

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

pub fn tauri_generate_context() -> tauri::Context {
    tauri::generate_context!()
}

#[pymodule(gil_used = false)]
#[pyo3(name = "ext_mod")]
pub mod ext_mod {
    use super::*;

    #[pymodule_init]
    fn init(module: &Bound<'_, PyModule>) -> PyResult<()> {
        pytauri::pymodule_export(
            module,
            // i.e., `context_factory` function of python binding
            |_args, _kwargs| Ok(tauri_generate_context()),
            // i.e., `builder_factory` function of python binding
            |_args, _kwargs| {
                let builder = tauri::Builder::default()
                    .plugin(tauri_plugin_opener::init())
                    .invoke_handler(tauri::generate_handler![greet]);
                Ok(builder)
            },
        )
    }
}
```

```rust title="src-tauri/src/main.rs"  linenums="1" hl_lines="67 72"
// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::env::var;

use pyo3::{prelude::*, wrap_pymodule};
use pytauri::standalone::{
    append_ext_mod, get_python_executable_from_venv, prepare_freethreaded_python_with_executable,
    write_py_err_to_file,
};
use tauri::{Builder, Manager as _};

use tauri_app_lib::{ext_mod, tauri_generate_context};

fn prepare_python_interpreter() {
    // `cfg(dev)` is set by `tauri-build` in `build.rs`, which means running with `tauri dev`,
    // see: <https://github.com/tauri-apps/tauri/pull/8937>.
    if cfg!(dev) {
        // virtualenv Python
        //
        // See:
        //
        // - <https://github.com/PyO3/pyo3/issues/3589>
        // - <https://github.com/PyO3/pyo3/issues/1896>
        //
        // pyo3 currently cannot automatically detect the virtual environment and configure pyconfig,
        // so we do it manually here.
        let venv_path = var("VIRTUAL_ENV").expect(
            "The app is running in tauri dev mode, \
                please activate the python virtual environment first \
                or set the `VIRTUAL_ENV` environment variable",
        );
        let python_executable = get_python_executable_from_venv(venv_path);
        prepare_freethreaded_python_with_executable(python_executable)
            .expect("failed to initialize python from venv");
    } else {
        // embedded Python, i.e., bundle mode with `tauri build`.

        // Actually, we don't use this app, we just use it to get the resource directory
        let sample_app = Builder::default()
            .build(tauri_generate_context())
            .expect("failed to build sample app");
        let resource_dir = sample_app
            .path()
            .resource_dir()
            .expect("failed to get resource dir");

        let py_executable = if cfg!(windows) {
            resource_dir.join("python.exe")
        } else {
            resource_dir.join("bin/python3")
        };

        debug_assert!(
            py_executable.is_file(),
            "Python executable not found, maybe you forgot to bundle it: {}",
            py_executable.display()
        );

        prepare_freethreaded_python_with_executable(py_executable)
            .expect("failed to initialize embedded python");
    }
}

fn execute_python_script(py: Python<'_>) -> PyResult<()> {
    // insert pytauri extension module into `sys.modules`
    append_ext_mod(wrap_pymodule!(ext_mod)(py).into_bound(py))?;

    // execute your Python script
    py.run(
        // equivalent to `python -m tauri_app`
        c"from runpy import run_module; run_module('tauri_app')",
        None,
        None,
    )
}

fn main() -> Result<(), PyErr> {
    prepare_python_interpreter();

    Python::with_gil(|py| {
        let result = execute_python_script(py);

        // handle the error
        result.inspect_err(|e| {
            if cfg!(all(not(debug_assertions), windows)) {
                // I.g., `windows_subsystem = "windows"` in `main.rs`.
                // In this case, there is no console to print the error, so we write the error to a file
                write_py_err_to_file(py, e, "error.log").expect("failed to write error to file");
            } else {
                // we have a console, so we just call `sys.excepthook` and print the error
                e.print_and_set_sys_last_vars(py);
            }
        })
    })
}
```

👆 Note the highlighted part, it will execute the code in `src-tauri/python/tauri_app/__main__.py`.

## Launch the app in dev mode

The `tauri-cli` has the ability to watch code changes and hot reload. Before starting, we need to add the following file to tell `tauri-cli` to ignore the python bytecode:

ref: <https://tauri.app/develop/#reacting-to-source-code-changes>

```gitignore title="src-tauri/.taurignore"
__pycache__
```

Also, we need tell `vite` to ignore `.venv`:

```ts title="vite.config.ts"
// https://vitejs.dev/config/
export default defineConfig(async () => ({
  server: {
    watch: {
      // 3. tell vite to ignore watching `src-tauri`
      ignored: ["**/src-tauri/**", "**/.venv/**"],
    },
  },
}));
```

Run `#!bash pnpm tauri dev`, and after recompiling, you will see a window similar to the previous step.

Try modifying the Python code, and you will notice that the Python code is quickly reloaded **without needing to recompile the Rust code**.

## Next Steps

Next, we will demonstrate how to package your application.
