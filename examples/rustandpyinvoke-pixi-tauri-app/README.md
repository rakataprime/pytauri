# Pytauri Integration Issue Report

## Issue Summary

We're encountering persistent issues with Python-Rust interop in our Pytauri integration, specifically around GIL management and async command handling. The application initializes successfully but fails during command execution.

## Environment Details

```bash
# Environment versions (please fill in your versions)
Python: 3.11.x
Rust: x.xx.x
Node: xx.x.x
OS: Linux 6.6.58-1-lts
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

# 3. Run via Python
python -m tauri_app
```

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
```
# Please attach your logs here when reporting
# Include logs from both Python and Rust launch attempts
```

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
   # Method 1: Python launch
   python -m tauri_app

   # Method 2: Rust launch
   cargo run
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