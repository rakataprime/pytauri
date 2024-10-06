from importlib.metadata import entry_points
from typing import Protocol, Union
from types import ModuleType

__all__ = ["py_invoke_handler", "EXT_MOD"]


def _load_ext_mod() -> ModuleType:
    _eps = entry_points(group="pytauri", name="ext_mod")
    if len(_eps) != 1:
        raise RuntimeError(
            f"Exactly one `pytauri` entry point is expected, but got {_eps!r}"
        )

    _ext_mod = _eps[0].load()
    assert isinstance(_ext_mod, ModuleType)
    return _ext_mod


def _load_pytauri_mod(ext_mod: ModuleType) -> ModuleType:
    try:
        _pytauri_mod = ext_mod.pytauri
    except AttributeError as e:
        raise RuntimeError(
            "Submodule `pytauri` is not found in the extension module"
        ) from e

    assert isinstance(_pytauri_mod, ModuleType)
    return _pytauri_mod


EXT_MOD = _load_ext_mod()

_pytauri_mod = _load_pytauri_mod(EXT_MOD)


_HandlerArgType = bytearray
# from `Vec<u8>`. See https://pyo3.rs/v0.22.2/conversions/tables#argument-types
_HandlerReturnType = Union[bytes, bytearray]


class _HandlerType(Protocol):
    def __call__(self, arg: _HandlerArgType, /) -> _HandlerReturnType: ...


class _PyInvokeHandlerType(Protocol):
    def __call__(self, func_name: str, py_func: _HandlerType) -> None: ...


py_invoke_handler: _PyInvokeHandlerType = _pytauri_mod.py_invoke_handler
