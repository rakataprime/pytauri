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
uv sync --package=pytauri-demo --reinstall
```

This will install the pytauri app and its Python dependencies in editable mode, meaning you don't need to reinstall the pytauri app throughout the development cycle. Then you can develop it like a normal Tauri application:

```bash
cargo tauri dev
# or: `pnpm tauri dev`, depends on how you install tauri-cli
```

## Release

> [!NOTE]
>
> Currently, we only support building standalone executables for Windows, Debian.
>
> MacOS will be supported soon.
>
> Issue [#2](https://github.com/WSH032/pytauri/issues/2).
>
> We welcome any PRs or suggestions.

### Get portable Python

We will bundle [python-build-standalone](https://github.com/indygreg/python-build-standalone) as a portable Python for distribution.

Please download the [Python version](https://github.com/indygreg/python-build-standalone/releases) you need. Usually, you will use these versions:

- `cpython-*-x86_64-pc-windows-msvc-install_only_stripped.tar.gz`
- `cpython-*-x86_64-unknown-linux-gnu-install_only_stripped.tar.gz`
- `cpython-*-x86_64-apple-darwin-install_only.tar.gz`

Extract it to `example\pyembed`.

Make sure the file layout is as follows:

Windows:

```tree
├── example\pyembed\python
    ├── python.exe
    ├── python3.dll
    └── ...
```

Unix:

```tree
├── example\pyembed\python
    ├── bin\
    ├── include\
    ├── lib\
    └── share\
```

### Install python lib dependencies

Use the following command to install the pytauri app itself and its Python dependencies into the embedded Python environment:

Windows:

```powershell
uv pip install --exact `
    --python=".\example\pyembed\python\python.exe" `
    --refresh `
    --no-sources `
    .\python\codelldb `
    .\python\pyo3-utils `
    .\python\pytauri `
    .\python\pytauri-plugin-notification `
    .\example
```

Unix:

```bash
uv pip install --exact \
    --python="./example/pyembed/python/bin/python3" \
    --refresh \
    --no-sources \
    ./python/codelldb \
    ./python/pyo3-utils \
    ./python/pytauri \
    ./python/pytauri-plugin-notification \
    ./example
```

> NOTE: Since we have not yet published the Python packages on PyPI, you need to install not only pytuari-demo in `.\example` but also all the packages in `.\python\*`.
>
> Once we publish these Python packages, you only need to install your own pytauri app.

<!-- This comment is to prevent markdownlint errors -->

> Currently, we only support installing dependencies at compile time. In the future, we will support dynamic installation of dependencies at runtime.

### configure tauri-cli

Create a new `.\example\Tauri.standalone.json`:

See: <https://tauri.app/reference/config/#bundle>

```json
{
    "bundle": {
        "active": true,
        "targets": "all",
        "resources": {
            "pyembed/python": "./"
        }
    }
}
```

> NOTE: It is in `json` format, not `toml` format. The current `--config` option of tauri-cli only supports `json` format.

### Build and bundle

Indicate to pyo3 to use the embedded Python interpreter through environment variables, so it does not mistakenly use the system Python interpreter.

Windows:

```powershell
$env:PYO3_PYTHON = "<Absolute Path>\example\pyembed\python\python.exe"
```

Unix:

```bash
export PYO3_PYTHON="<Absolute Path>/example/pyembed/python/bin/python3"
```

Finally, use tauri-cli to bundle:

Windows:

```powershell
cargo tauri build --config=".\example\Tauri.standalone.json"
```

Debian:

```bash
# - After installation, the embedded Python will be located in `/usr/lib/pytauri-demo/`
# - `/pyembed/python/lib` is not in the default search path, so `-L` is needed to specify it
# - `pytauri-demo` is your app name.
export RUSTFLAGS=" \
    -C link-arg=-Wl,-rpath,/usr/lib/pytauri-demo/lib \
    -L <Absolute Path>/example/pyembed/python/lib"
cargo tauri build --config="./example/Tauri.standalone.json"
```

> NOTE:
>
> Do not use `Tauri.toml` or `Tauri.windows.toml` directly.
>
> The current tauri-cli will copy `bundle.resources` to `target/release(debug)`, which is in the same location as your executable. This means the copied Python environment will affect the Python environment linked at runtime during `cargo run`/`cargo tauri dev`.
>
> Manually specifying the configuration file with the `cargo tauri build --config` option ensures that only `target/release` is affected. Although this is not perfect, it is a compromise that at least ensures the `cargo tauri dev` development experience. We need to contact the tauri team to solve this problem.

Now go to `target/release/bundle` to collect your application 😉.
