# Pytauri Integration Issue Report

## Issue Summary

We're encountering persistent issues with Python-Rust interop in our Pytauri integration, specifically around GIL management and async command handling. The application initializes successfully but fails during command execution.

## Environment Details

```bash
# Environment versions (please fill in your versions)
Python: 3.10.x
Rust: 1.84.1
Node: v18.20.6
OS: Linux 6.6.58-1-lts (arch)
```

### Dependencies
- pytauri: 0.2.*
- pydantic: 2.*
- anyio: 4.*
- tauri: 2.2
- pyo3: 0.23

## Setup Steps Taken

### Python Launch Method
```bash
# 1. Clone and setup frontend
git clone <repo_url>
cd <repo_dir>
pnpm install
pnpm -r run build

# 2. Setup Python environment
uv venv
source .venv/bin/activate  # or .venv\Scripts\Activate.ps1 on Windows
uv pip install --reinstall -e src-tauri

# 3. Run via Tauri
RUST_BACKTRACE=1 RUST_LOG=debug VIRTUAL_ENV=$(pwd)/src-tauri/.venv pnpm tauri dev```

### Rust Launch Method
We're also trying to launch directly from Rust. Key initialization code:

```rust
// 1. Initialize Python interpreter
let py_script = PythonScript::Module("tauri_app".into());
let pybuilder = PythonInterpreterBuilder::new(py_env, py_script, |py| {
    debug!("Initializing Python extension module");
    wrap_pymodule!(ext_mod)(py)
});

let interpreter = pybuilder.build()?;

// 2. Get Python invoke handler
let py_invoke_handler: PyObject = Python::with_gil(|py| {
    py.import("tauri_app")?
        .getattr("commands")?
        .into_py(py)
});

// 3. Launch Tauri application
Builder::default()
    .plugin(tauri_plugin_pytauri::init(py_invoke_handler))  // Initialize Python plugin
    .setup(|app| {
        app.manage(interpreter);  // Manage interpreter lifecycle
        // ... other setup
        Ok(())
    })
    .invoke_handler(tauri::generate_handler![tauri_app_lib::greet_rust])
    .run(generate_context!())?;
```

**Key Differences from Python Launch**:
1. Interpreter lifecycle managed by Rust
2. Manual GIL management during initialization
3. Direct plugin initialization with invoke handler
4. Interpreter stored in Tauri state management

## Debugging Journey

### 1. Initial Error
```
Error in tauri-plugin-pytauri::commands::pyfunc at line 37
```
- Application fails during command execution
- Python interpreter initializes successfully
- Command registration appears to work
- Error occurs in both Python and Rust launch methods

### 2. Attempted Solutions

#### First Implementation
```python
@commands.command()
def greet(body: bytes) -> bytes:
    try:
        data = json.loads(body)
        name = data.get('name', 'World')
        return json.dumps({"message": f"Hello, {name}!"}).encode()
    except Exception as e:
        return json.dumps({"error": str(e)}).encode()
```
**Result**: Failed with GIL-related errors

#### Added Logging
```python
logging.basicConfig(
    level=logging.DEBUG,
    format='%(asctime)s - %(name)s - %(levelname)s - %(message)s'
)
logger = logging.getLogger(__name__)

def _safe_greet(body_bytes: bytes) -> bytes:
    try:
        logger.debug(f"Processing request: {body_bytes}")
        # Implementation
    except Exception as e:
        logger.error(f"Error: {e}")
        return default_response
```
**Result**: Revealed GIL management/release issues

#### Added Type Safety
```python
class GreetRequest(BaseModel):
    name: Optional[str] = None

def greet(body: Dict[str, Any]) -> bytes:
    request = GreetRequest.parse_obj(body or {})
```
**Result**: Data validation works but GIL issues persist

#### Current Implementation
```python
def sync_greet(body: Dict[str, Any]) -> bytes:
    """Synchronous part that runs with GIL held"""
    try:
        logger.debug(f"Received body: {body}")
        request = GreetRequest.parse_obj(body or {})
        name = request.name or "World"
        response = {"message": f"Hello, {name}!"}
        return json.dumps(response).encode()
    except Exception as e:
        logger.error(f"Error in sync_greet: {e}")
        return json.dumps({"error": str(e)}).encode()

async def greet(body: Dict[str, Any]) -> bytes:
    """Async wrapper ensuring proper GIL handling"""
    try:
        loop = asyncio.get_running_loop()
        return await loop.run_in_executor(None, sync_greet, body)
    except Exception as e:
        logger.error(f"Error in async greet: {e}")
        return json.dumps({"error": str(e)}).encode()
```
**Result**: Still encountering GIL management issues

## Current Issues

1. **Primary Issue**: GIL management in async contexts
   - Commands fail during execution
   - Error occurs in Pytauri GIL runtime thread
   - Issue persists in both launch methods

2. **Secondary Issues**:
   - Exception propagation between Python and Rust layers
   - Portal initialization timing
   - Command registration synchronization
   - Potential differences in GIL handling between launch methods

## Debug Information

### Logs
```bash
2025-02-19T20:50:06Z DEBUG tauri_app] Running in dev mode
[2025-02-19T20:50:06Z INFO  tauri_app] Using virtual environment at: /home/Documents/thumper/crew/tr/pytauri/examples/rustandpyinvoke-tauri-app/src-tauri/.venv
[2025-02-19T20:50:06Z INFO  tauri_app] Setting up Python interpreter...
[2025-02-19T20:50:06Z DEBUG tauri_app] Initializing Python extension module
[2025-02-19T20:50:06Z INFO  tauri_app] Python interpreter built successfully
[2025-02-19T20:50:06Z INFO  tauri_app] Setting up Python invoke handler...
[2025-02-19T20:50:06Z DEBUG tauri_app] Acquiring Python GIL
[2025-02-19T20:50:06Z DEBUG tauri_app] Successfully imported tauri_app module
[2025-02-19T20:50:06Z DEBUG tauri_app] Successfully got commands object
[2025-02-19T20:50:06Z INFO  tauri_app] Initializing Tauri application...
[2025-02-19T20:50:07Z WARN  tauri_utils] `APPDIR` or `APPIMAGE` environment variable found but this application was not detected as an AppImage; this might be a security issue.
[2025-02-19T20:50:07Z DEBUG tauri_app] Running Tauri setup
[/etc/pulse/client.conf:37] Unknown lvalue 'default-sample-rate' in section 'n/a'.
Exception ignored in: {'greet': _PyInvokHandleData(parameters=mappingproxy(OrderedDict({'body': <Parameter "body: bytes">, 'app_handle': <Parameter "app_handle: AppHandle">})), handler=<function greet at 0x769222586a20>)}
RuntimeError: `py_invoke_handler` raised an exception
thread 'tauri-plugin-pytauri-gil-rt' panicked at /home//Documents/thumper/crew/tr/pytauri/crates/tauri-plugin-pytauri/src/commands.rs:37:13:
`py_invoke_handler` shouldn't raise exception
stack backtrace:
   0: rust_begin_unwind
             at /rustc/e71f9a9a98b0faf423844bf0ba7438f29dc27d58/library/std/src/panicking.rs:665:5
   1: core::panicking::panic_fmt
             at /rustc/e71f9a9a98b0faf423844bf0ba7438f29dc27d58/library/core/src/panicking.rs:76:14
   2: tauri_plugin_pytauri::commands::pyfunc::{{closure}}
             at /home//Documents/thumper/crew/tr/pytauri/crates/tauri-plugin-pytauri/src/commands.rs:37:13
   3: pyo3::marker::Python::with_gil
             at /home//.cargo/registry/src/index.crates.io-6f17d22bba15001f/pyo3-0.23.4/src/marker.rs:412:9
   4: tauri_plugin_pytauri::gil_runtime::task_with_gil::{{closure}}
             at /home//Documents/thumper/crew/tr/pytauri/crates/tauri-plugin-pytauri/src/gil_runtime.rs:48:31
   5: tokio::runtime::task::core::Core<T,S>::poll::{{closure}}
             at /home//.cargo/registry/src/index.crates.io-6f17d22bba15001f/tokio-1.43.0/src/runtime/task/core.rs:331:17
   6: tokio::loom::std::unsafe_cell::UnsafeCell<T>::with_mut
             at /home//.cargo/registry/src/index.crates.io-6f17d22bba15001f/tokio-1.43.0/src/loom/std/unsafe_cell.rs:16:9
   7: tokio::runtime::task::core::Core<T,S>::poll
             at /home//.cargo/registry/src/index.crates.io-6f17d22bba15001f/tokio-1.43.0/src/runtime/task/core.rs:320:13
   8: tokio::runtime::task::harness::poll_future::{{closure}}
             at /home//.cargo/registry/src/index.crates.io-6f17d22bba15001f/tokio-1.43.0/src/runtime/task/harness.rs:532:19
   9: <core::panic::unwind_safe::AssertUnwindSafe<F> as core::ops::function::FnOnce<()>>::call_once
             at /rustc/e71f9a9a98b0faf423844bf0ba7438f29dc27d58/library/core/src/panic/unwind_safe.rs:272:9
  10: std::panicking::try::do_call
             at /rustc/e71f9a9a98b0faf423844bf0ba7438f29dc27d58/library/std/src/panicking.rs:557:40
  11: __rust_try
  12: std::panicking::try
             at /rustc/e71f9a9a98b0faf423844bf0ba7438f29dc27d58/library/std/src/panicking.rs:520:19
  13: std::panic::catch_unwind
             at /rustc/e71f9a9a98b0faf423844bf0ba7438f29dc27d58/library/std/src/panic.rs:358:14
  14: tokio::runtime::task::harness::poll_future
             at /home//.cargo/registry/src/index.crates.io-6f17d22bba15001f/tokio-1.43.0/src/runtime/task/harness.rs:520:18
  15: tokio::runtime::task::harness::Harness<T,S>::poll_inner
             at /home//.cargo/registry/src/index.crates.io-6f17d22bba15001f/tokio-1.43.0/src/runtime/task/harness.rs:209:27
  16: tokio::runtime::task::harness::Harness<T,S>::poll
             at /home//.cargo/registry/src/index.crates.io-6f17d22bba15001f/tokio-1.43.0/src/runtime/task/harness.rs:154:15
  17: tokio::runtime::task::raw::poll
             at /home//.cargo/registry/src/index.crates.io-6f17d22bba15001f/tokio-1.43.0/src/runtime/task/raw.rs:271:5
  18: tokio::runtime::task::raw::RawTask::poll
             at /home//.cargo/registry/src/index.crates.io-6f17d22bba15001f/tokio-1.43.0/src/runtime/task/raw.rs:201:18
  19: tokio::runtime::task::LocalNotified<S>::run
             at /home//.cargo/registry/src/index.crates.io-6f17d22bba15001f/tokio-1.43.0/src/runtime/task/mod.rs:449:9
  20: tokio::runtime::scheduler::multi_thread::worker::Context::run_task::{{closure}}
             at /home//.cargo/registry/src/index.crates.io-6f17d22bba15001f/tokio-1.43.0/src/runtime/scheduler/multi_thread/worker.rs:596:13
  21: tokio::runtime::coop::with_budget
             at /home//.cargo/registry/src/index.crates.io-6f17d22bba15001f/tokio-1.43.0/src/runtime/coop.rs:107:5
  22: tokio::runtime::coop::budget
             at /home//.cargo/registry/src/index.crates.io-6f17d22bba15001f/tokio-1.43.0/src/runtime/coop.rs:73:5
  23: tokio::runtime::scheduler::multi_thread::worker::Context::run_task
             at /home//.cargo/registry/src/index.crates.io-6f17d22bba15001f/tokio-1.43.0/src/runtime/scheduler/multi_thread/worker.rs:595:9
  24: tokio::runtime::scheduler::multi_thread::worker::Context::run
             at /home//.cargo/registry/src/index.crates.io-6f17d22bba15001f/tokio-1.43.0/src/runtime/scheduler/multi_thread/worker.rs:546:24
  25: tokio::runtime::scheduler::multi_thread::worker::run::{{closure}}::{{closure}}
             at /home//.cargo/registry/src/index.crates.io-6f17d22bba15001f/tokio-1.43.0/src/runtime/scheduler/multi_thread/worker.rs:511:21
  26: tokio::runtime::context::scoped::Scoped<T>::set
             at /home//.cargo/registry/src/index.crates.io-6f17d22bba15001f/tokio-1.43.0/src/runtime/context/scoped.rs:40:9
  27: tokio::runtime::context::set_scheduler::{{closure}}
             at /home//.cargo/registry/src/index.crates.io-6f17d22bba15001f/tokio-1.43.0/src/runtime/context.rs:180:26
  28: std::thread::local::LocalKey<T>::try_with
             at /rustc/e71f9a9a98b0faf423844bf0ba7438f29dc27d58/library/std/src/thread/local.rs:283:12
  29: std::thread::local::LocalKey<T>::with
             at /rustc/e71f9a9a98b0faf423844bf0ba7438f29dc27d58/library/std/src/thread/local.rs:260:9
  30: tokio::runtime::context::set_scheduler
             at /home//.cargo/registry/src/index.crates.io-6f17d22bba15001f/tokio-1.43.0/src/runtime/context.rs:180:9
  31: tokio::runtime::scheduler::multi_thread::worker::run::{{closure}}
             at /home//.cargo/registry/src/index.crates.io-6f17d22bba15001f/tokio-1.43.0/src/runtime/scheduler/multi_thread/worker.rs:506:9
  32: tokio::runtime::context::runtime::enter_runtime
             at /home//.cargo/registry/src/index.crates.io-6f17d22bba15001f/tokio-1.43.0/src/runtime/context/runtime.rs:65:16
  33: tokio::runtime::scheduler::multi_thread::worker::run
             at /home//.cargo/registry/src/index.crates.io-6f17d22bba15001f/tokio-1.43.0/src/runtime/scheduler/multi_thread/worker.rs:498:5
  34: tokio::runtime::scheduler::multi_thread::worker::Launch::launch::{{closure}}
             at /home//.cargo/registry/src/index.crates.io-6f17d22bba15001f/tokio-1.43.0/src/runtime/scheduler/multi_thread/worker.rs:464:45
  35: <tokio::runtime::blocking::task::BlockingTask<T> as core::future::future::Future>::poll
             at /home//.cargo/registry/src/index.crates.io-6f17d22bba15001f/tokio-1.43.0/src/runtime/blocking/task.rs:42:21
  36: tokio::runtime::task::core::Core<T,S>::poll::{{closure}}
             at /home//.cargo/registry/src/index.crates.io-6f17d22bba15001f/tokio-1.43.0/src/runtime/task/core.rs:331:17
  37: tokio::loom::std::unsafe_cell::UnsafeCell<T>::with_mut
             at /home//.cargo/registry/src/index.crates.io-6f17d22bba15001f/tokio-1.43.0/src/loom/std/unsafe_cell.rs:16:9
  38: tokio::runtime::task::core::Core<T,S>::poll
             at /home//.cargo/registry/src/index.crates.io-6f17d22bba15001f/tokio-1.43.0/src/runtime/task/core.rs:320:13
  39: tokio::runtime::task::harness::poll_future::{{closure}}
             at /home//.cargo/registry/src/index.crates.io-6f17d22bba15001f/tokio-1.43.0/src/runtime/task/harness.rs:532:19
  40: <core::panic::unwind_safe::AssertUnwindSafe<F> as core::ops::function::FnOnce<()>>::call_once
             at /rustc/e71f9a9a98b0faf423844bf0ba7438f29dc27d58/library/core/src/panic/unwind_safe.rs:272:9
  41: std::panicking::try::do_call
             at /rustc/e71f9a9a98b0faf423844bf0ba7438f29dc27d58/library/std/src/panicking.rs:557:40
  42: __rust_try
  43: std::panicking::try
             at /rustc/e71f9a9a98b0faf423844bf0ba7438f29dc27d58/library/std/src/panicking.rs:520:19
  44: std::panic::catch_unwind
             at /rustc/e71f9a9a98b0faf423844bf0ba7438f29dc27d58/library/std/src/panic.rs:358:14
  45: tokio::runtime::task::harness::poll_future
             at /home//.cargo/registry/src/index.crates.io-6f17d22bba15001f/tokio-1.43.0/src/runtime/task/harness.rs:520:18
  46: tokio::runtime::task::harness::Harness<T,S>::poll_inner
             at /home//.cargo/registry/src/index.crates.io-6f17d22bba15001f/tokio-1.43.0/src/runtime/task/harness.rs:209:27
  47: tokio::runtime::task::harness::Harness<T,S>::poll
             at /home//.cargo/registry/src/index.crates.io-6f17d22bba15001f/tokio-1.43.0/src/runtime/task/harness.rs:154:15
  48: tokio::runtime::task::raw::poll
             at /home//.cargo/registry/src/index.crates.io-6f17d22bba15001f/tokio-1.43.0/src/runtime/task/raw.rs:271:5
  49: tokio::runtime::task::raw::RawTask::poll
             at /home//.cargo/registry/src/index.crates.io-6f17d22bba15001f/tokio-1.43.0/src/runtime/task/raw.rs:201:18
  50: tokio::runtime::task::UnownedTask<S>::run
             at /home//.cargo/registry/src/index.crates.io-6f17d22bba15001f/tokio-1.43.0/src/runtime/task/mod.rs:486:9
  51: tokio::runtime::blocking::pool::Task::run
             at /home//.cargo/registry/src/index.crates.io-6f17d22bba15001f/tokio-1.43.0/src/runtime/blocking/pool.rs:161:9
  52: tokio::runtime::blocking::pool::Inner::run
             at /home//.cargo/registry/src/index.crates.io-6f17d22bba15001f/tokio-1.43.0/src/runtime/blocking/pool.rs:511:17
  53: tokio::runtime::blocking::pool::Spawner::spawn_thread::{{closure}}
             at /home//.cargo/registry/src/index.crates.io-6f17d22bba15001f/tokio-1.43.0/src/runtime/blocking/pool.rs:469:13
note: Some details are omitted, run with `RUST_BACKTRACE=full` for a verbose backtrace.```

### Relevant Files
Please check these files match your implementation:
- `src-tauri/python/tauri_app/__init__.py`
- `src-tauri/src/main.rs` (especially the initialization code)
- `src-tauri/src/lib.rs`

## How to Reproduce

1. Set up environment:
   ```bash
   export RUST_LOG=debug
   export PYTHONVERBOSE=1
   ```

2. Run the application (try both methods):
   ```bash
RUST_BACKTRACE=1 RUST_LOG=debug VIRTUAL_ENV=$(pwd)/src-tauri/.venv pnpm tauri dev```
   ```

3. Attempt to use the greet command from the UI

## Questions for Maintainers

1. Is our GIL management approach in the async wrapper correct?
2. Should we be using a different executor configuration?
3. Are there known issues with portal initialization timing?
4. Are there recommended patterns for command registration in async contexts?
5. Are there different considerations for Python vs Rust launch methods?
6. Should the interpreter lifecycle management differ between launch methods?

## References

- [Pytauri Documentation](https://github.com/WSH032/pytauri)
- [Tauri Plugin Python Documentation](https://github.com/WSH032/pytauri/tree/main/crates/tauri-plugin-pytauri)
- [PyO3 GIL Management Guide](https://pyo3.rs/v0.20.0/python_from_rust.html#gil-management) 
