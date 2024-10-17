# PyTauri

[Tauri] bindings for Python through [Pyo3].

[Tauri]: https://github.com/tauri-apps/tauri
[Pyo3]: https://github.com/PyO3/pyo3

> [!NOTE]
>
> WIP: Currently we only support local development.
>
> Once we publish packages to `PyPi`, `Crates.io`, `npm`, you can use `pytauri` for production.

## Features

[notification]: https://docs.rs/tauri-plugin-notification/latest/tauri_plugin_notification/

- Need Rust compiler, but **don't need to write Rust code**!
- No IPC (inter-process communication) overhead, secure and fast, thanks to [Pyo3]!
- Support Tauri official plugins(e.g., [notification]), and you can write your own plugins!

    ![demo](https://github.com/user-attachments/assets/14ad5b51-b333-4d80-b04b-af72c4179571)

- Ergonomic API (and as close as possible to the Tauri Rust API)

    - Python

        ```python
        from pydantic import BaseModel
        from pytauri import py_invoke_handler, AppHandle
        from pytauri_plugin_notification import NotificationExt

        class Person(BaseModel):
            name: str


        class Greeting(BaseModel):
            message: str


        @py_invoke_handler()
        def greet(person: Person, app_handle: AppHandle) -> Greeting:
            notification_ext = NotificationExt(app_handle)
            notification = notification_ext.notification()
            notification.builder().title("Greeting").body(f"Hello, {person.name}!").show()

            return Greeting(message=f"Hello, {person.name}!")
        ```

    - Frontend

        ```tsx
        import { pyInvoke, fromJson } from "tauri-plugin-pytauri-api";

        export interface Person {
            name: string;
        }

        export interface Greeting {
            message: string;
        }

        export async function greet(person: Person): Promise<Greeting> {
            const response = await pyInvoke("greet", person);
            return fromJson(response);
        }
        ```

## Early Access

### Developer Requirements

#### Platforms

- Tier 1: my primary development environment
    - Windows 10
- Tier 2: will got bugs fixed and tested
    - Linux
- Tier 3: will not be tested, may not work
    - MacOS

#### Language

- [Python]: >= 3.9
- [Rust]: The latest stable version
- [Node.js]: LTS version

[Rust]: https://www.rust-lang.org/tools/install
[Python]: https://www.python.org/downloads/
[Node.js]: https://nodejs.org/en/download/package-manager

#### Tools

- [pnpm]: See `package.json`
- [uv]: The latest version

[pnpm]: https://pnpm.io/installation
[uv]: https://docs.astral.sh/uv/getting-started/installation/

#### System Dependencies

- [Tauri Prerequisites](https://tauri.app/start/prerequisites/#system-dependencies)

### Install

```bash
git clone https://github.com/WSH032/pytauri.git
cd pytauri

pnpm install
# build frontend assets
pnpm -r run build

# virtual environment
uv venv
source .venv/bin/activate  # bash/zsh
# or powershell: .venv\Scripts\Activate.ps1

uv pip install setuptools setuptools-rust setuptools-scm
# install demo
uv sync \
    --no-build-isolation-package=pytauri-demo \
    --reinstall-package=pytauri-demo
```

### Usage

```bash
python -m pytauri_demo
```

### Example

See `example`

- Backend
    - Python: `example\python\pytauri_demo\__main__.py`
    - Rust: `example\src\lib.rs`
- Frontend: `example\front\src\ipc.tsx`

## Philosophy

### For Pythoneer

I hope `PyTauri` can become an alternative to [pywebview] and [Pystray], leveraging Tauri's comprehensive features to offer Python developers a GUI framework and a batteries-included development experience similar to [electron] and [PySide].

> PyTauri is inspired by [FastAPI] and [Pydantic], aiming to offer a similar development experience.

### For Rustacean

Through [Pyo3], I hope Rust developers can better utilize the Python ecosystem (e.g., building AI GUI applications with [PyTorch]).

Although Rust's lifetime and ownership system makes Rust code safer, Python's garbage collection (GC) will make life easier. 😆

### The Future

In the future, I hope PyTauri can integrate with [nicegui] and [gradio], bringing you a Python full-stack (i.g., without `Node.js`) development experience.

[pywebview]: https://github.com/r0x0r/pywebview
[Pystray]: https://github.com/moses-palmer/pystray
[electron]: https://github.com/electron/electron
[PySide]: https://wiki.qt.io/Qt_for_Python
[FastAPI]: https://github.com/fastapi/fastapi
[Pydantic]: https://github.com/pydantic/pydantic
[PyTorch]: https://github.com/pytorch/pytorch
[nicegui]: https://github.com/zauberzeug/nicegui
[gradio]: https://github.com/gradio-app/gradio

## Credits

PyTauri is a project that aims to provide Python bindings for [Tauri], a cross-platform webview GUI library. `Tauri` is a trademark of the Tauri Program within the Commons Conservancy and PyTauri is not officially endorsed or supported by them. PyTauri is an independent and community-driven effort that respects the original goals and values of Tauri. PyTauri does not claim any ownership or affiliation with the Tauri Program.

## License

This project is licensed under the terms of the *Apache License 2.0*.
