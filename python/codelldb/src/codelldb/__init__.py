# NOTE: DO NOT use third-party libraries in this file,
# keep the minimal dependencies.

"""Launch [CodeLLDB] in [debugpy] to debug rust code.

[CodeLLDB]: https://marketplace.visualstudio.com/items?itemName=vadimcn.vscode-lldb
[debugpy]: https://marketplace.visualstudio.com/items?itemName=ms-python.debugpy

# Usage

!!! tip

    This package only depends on the Python standard library, so you can integrate it freely.

## Configuring CodeLLDB rpc server

Please refer to the CodeLLDB documentation to set up the [rpc server](https://github.com/vadimcn/codelldb/blob/master/MANUAL.md#rpc-server),
and add the following content to `.vscode/settings.json`:

```json
{
  "lldb.rpcServer": {
    "host": "127.0.0.1",
    "port": 9552,
    "token": "secret",
  }
}
```

## Configuring Python Debugger

Please refer to the Python Debugger [documentation](https://code.visualstudio.com/docs/python/debugging),
and add the following content to `.vscode/launch.json`:

```json
{
    "version": "0.2.0",
    "configurations": [
        {
            "name": "python.debug",
            "type": "debugpy",
            "purpose": [
                "debug-in-terminal"
            ],
            "console": "integratedTerminal",
            "request": "launch",
            // see: <https://code.visualstudio.com/docs/python/debugging#_set-configuration-options>
            "module": "pytauri_demo", // your module name
            "args": [], // whatever you want
            // following env vars will be used by `codelldb` package
            "env": {
                "VSCODE_RUST_DEBUG": "${input:rustDebug}",
                // keep the following env vars in sync with `.vscode/settings.json`
                "VSCODE_LLDB_RPC_SERVER_HOST": "127.0.0.1",
                "VSCODE_LLDB_RPC_SERVER_PORT": "9552",
                "VSCODE_LLDB_RPC_SERVER_TOKEN": "secret",
            },
        }
    ],
    // see: <https://code.visualstudio.com/docs/editor/variables-reference#_input-variables>
    "inputs": [
        {
            "type": "pickString",
            "id": "rustDebug",
            "description": "Whether to enable Rust debugging",
            "options": [
                {
                    "label": "Enabled",
                    "value": "1"
                },
                {
                    "label": "Disabled",
                    "value": "0"
                }
            ],
            "default": "0"
        },
    ]
}
```

## [Multi-target debugging](https://code.visualstudio.com/docs/editor/debugging#_multitarget-debugging)

When starting Python Debugger via `F5`, you will see a selection box, choose `Enabled` to enable Rust debugging.

---
"""

import json
import socket
from logging import getLogger
from os import getenv, getpid
from textwrap import dedent
from typing import Optional, TypedDict

__all__ = ["debug"]

VSCODE_RUST_DEBUG_VARNAME = "VSCODE_RUST_DEBUG"
"""Whether to enable Rust debugging, set to `1` to enable, `0` to disable."""
VSCODE_LLDB_RPC_SERVER_HOST_VARNAME = "VSCODE_LLDB_RPC_SERVER_HOST"
"""The host of the CodeLLDB rpc server."""
VSCODE_LLDB_RPC_SERVER_PORT_VARNAME = "VSCODE_LLDB_RPC_SERVER_PORT"
"""The port of the CodeLLDB rpc server."""
VSCODE_LLDB_RPC_SERVER_TOKEN_VARNAME = "VSCODE_LLDB_RPC_SERVER_TOKEN"
"""The secret token of the CodeLLDB rpc server."""


class _LldbRpcServerCfg(TypedDict):
    host: str
    port: int
    token: Optional[str]


DEFAULT_HOST = "127.0.0.1"

_logger = getLogger(__name__)


class DebugError(Exception):
    pass


def _getenv_strict(env_var_name: str) -> str:
    value = getenv(env_var_name)
    if value is None:
        raise DebugError(f"`{env_var_name}` environment variable is not set")
    return value


def _get_lldb_rpc_server_cfg() -> _LldbRpcServerCfg:
    host = getenv(VSCODE_LLDB_RPC_SERVER_HOST_VARNAME) or DEFAULT_HOST
    port = _getenv_strict(VSCODE_LLDB_RPC_SERVER_PORT_VARNAME)
    try:
        port = int(port)
    except ValueError as e:
        raise DebugError(
            f"`{VSCODE_LLDB_RPC_SERVER_PORT_VARNAME}` environment variable is not a valid integer"
        ) from e
    token = getenv(VSCODE_LLDB_RPC_SERVER_TOKEN_VARNAME)

    return _LldbRpcServerCfg(host=host, port=port, token=token)


def debug() -> None:
    """Launch CodeLLDB to debug rust code.

    If the value indicated by [VSCODE_RUST_DEBUG_VARNAME][codelldb.VSCODE_RUST_DEBUG_VARNAME] is not `1`, this is a no-op.

    Raises:
        DebugError: If the provided environment variables cannot be correctly parsed.

    Note:
        If failed to connect to the lldb rpc server, a warning message will be logged instead of raising an exception.
    """
    vscode_rust_debug = getenv(VSCODE_RUST_DEBUG_VARNAME)
    if vscode_rust_debug != "1":
        return

    _logger.info(f"'{VSCODE_RUST_DEBUG_VARNAME}' is set, enabling rust debug mode")

    lldb_rpc_server_cfg = _get_lldb_rpc_server_cfg()

    host = lldb_rpc_server_cfg["host"]
    port = lldb_rpc_server_cfg["port"]
    token = lldb_rpc_server_cfg["token"]

    token_data = f"token: {token}" if token else ""
    # See: <https://github.com/vadimcn/codelldb/blob/v1.10.0/MANUAL.md#rpc-server>
    # Line-oriented YAML Syntax: <https://github.com/vadimcn/codelldb/blob/v1.10.0/MANUAL.md#debugging-externally-launched-code>
    # Arg: <https://github.com/vadimcn/codelldb/blob/v1.10.0/MANUAL.md#attaching-to-a-running-process>
    rpc_data = dedent(f"""\
        name: "rust.debug"
        type: "lldb"
        request: "attach"
        pid: {getpid()}
        sourceLanguages:
            - rust
            - c
            - cpp
        {token_data}
    """)

    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as s:
        s.connect((host, port))

        s.sendall(rpc_data.encode("utf-8"))

        s.shutdown(socket.SHUT_WR)

        response = s.recv(1024)

        if not response:
            _logger.warning(
                "Failed to get response from lldb rpc server, "
                "maybe the rpc `token` is not correct."
            )
            return

        try:
            response = json.loads(response)
            assert isinstance(response, dict)
        except Exception as e:
            _logger.warning(
                f"Failed to parse response from lldb rpc server: {response}", exc_info=e
            )
            return

        if response.get("success") is not True:  # pyright: ignore[reportUnknownMemberType]
            _logger.warning(
                f"Seems like lldb rpc server failed to attach to the process: {response}"
            )
            return
