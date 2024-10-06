from typing import Callable

from pytauri.ffi import py_invoke_handler
from pytauri_demo._ext_mod import run  # pyright: ignore[reportUnknownVariableType]

run: Callable[[], None]

__all__ = ["run", "py_invoke_handler"]
