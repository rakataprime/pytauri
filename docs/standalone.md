# Build standalone binary

pytauri can be distributed as a Python wheel or compiled into a standalone executable (a regular Tauri application).

Unlike packaging with `pyinstaller` after building as a wheel, compiling pytauri into an executable allows you to enjoy all the benefits brought by tauri-cli.

This tutorial assumes you have read the `README.md` and understand how to install pytauri in wheel format.

Make sure you have installed [tauri-cli](https://tauri.app/reference/cli/).

You should check `example/src/main.rs` to know how we implement the standalone executable.

## Development

Make sure you have activated the Python virtual environment, which will automatically set the `VIRTUAL_ENV` environment variable to indicate using the Python interpreter in the virtual environment.

Change the installation command to:

```bash
export PYTAURI_STANDALONE=1  # see `example/setup.py`
# or powershell: $env:PYTAURI_STANDALONE=1
uv sync --package=pytauri-demo
```

This will install the pytauri app and its Python dependencies in editable mode, meaning you don't need to reinstall the pytauri app throughout the development cycle. Then you can develop it like a normal Tauri application:

```bash
cargo tauri dev
# or: `pnpm tauri dev`, depends on how you install tauri-cli
```

## Release

> [!NOTE]
>
> Currently, we only support building standalone executables on Windows.
> This is because building standalone executables requires bundling the Python interpreter, and I am not sure how to achieve this on Linux and macOS.
>
> Issue [#2](https://github.com/WSH032/pytauri/issues/2).
>
> We welcome any PRs or suggestions.

### Windows

We will bundle [python-build-standalone](https://github.com/indygreg/python-build-standalone) as a portable Python for distribution.

Please download the [Python version](https://github.com/indygreg/python-build-standalone/releases) you need. Usually, `cpython-*-x86_64-pc-windows-msvc-install_only_stripped.tar.gz` is what you want.

Extract it to `example\pyembed`.

Make sure the file layout is as follows:

```tree
├── example\pyembed
    ├── python.exe
    ├── python3.dll
    └── ...
```

Use the following command to install the pytauri app itself and its Python dependencies into the embedded Python environment:

```powershell
uv pip install --exact `
    --python=".\example\pyembed\python.exe" `
    --refresh `
    --no-sources `
    .\python\codelldb `
    .\python\pyo3-utils `
    .\python\pytauri `
    .\python\pytauri-plugin-notification `
    .\example
```

> NOTE: Since we have not yet published the Python packages on PyPI, you need to install not only `.\example\pytuari-demo` but also all the packages in `.\python\*`.
>
> Once we publish these Python packages, you only need to install your own pytauri app.

<!-- This comment is to prevent markdownlint errors -->

> Currently, we only support installing dependencies at compile time. In the future, we will support dynamic installation of dependencies at runtime.

Create a new `.\example\Tauri.windows.standalone.json`:

See: <https://tauri.app/reference/config/#bundle>

```json
{
    "bundle": {
        "active": true,
        "targets": "all",
        "resources": {
            "pyembed/": "./"
        }
    }
}
```

> NOTE: It is in `json` format, not `toml` format. The current `--config` option of tauri-cli only supports `json` format.

Finally, use tauri-cli to bundle:

```powershell
# instruct pyo3 to use the embedded Python interpreter
$env:PYO3_PYTHON = "<Absolute Path>\example\pyembed\python.exe"
cargo tauri build --config=".\example\Tauri.windows.standalone.json"
```

> NOTE:
>
> Do not use `Tauri.toml` or `Tauri.windows.toml` directly.
>
> The current tauri-cli will copy `bundle.resources` to `target/release(debug)`, which is in the same location as your executable. This means the copied Python environment will affect the Python environment linked at runtime during `cargo run`/`cargo tauri dev`.
>
> Manually specifying the configuration file with the `cargo tauri build --config` option ensures that only `target/release` is affected. Although this is not perfect, it is a compromise that at least ensures the `cargo tauri dev` development experience. We need to contact the tauri team to solve this problem.

Now go to `target/release/bundle` to collect your application 😉.
