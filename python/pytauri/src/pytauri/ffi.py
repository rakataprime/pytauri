from importlib.metadata import entry_points
from typing import Protocol, Union
from types import ModuleType

__all__ = ["raw_invoke_handler", "EXT_MOD"]


def _load_ext_mod() -> ModuleType:
    eps = entry_points(group="pytauri", name="ext_mod")
    if len(eps) != 1:
        raise RuntimeError(
            f"Exactly one `pytauri` entry point is expected, but got {eps!r}"
        )

    # See: https://docs.python.org/3/library/importlib.metadata.html#entry-points
    # Changed in version 3.13: EntryPoint objects no longer present a tuple-like interface (__getitem__()).
    # If use `eps[0]` directly, pyright will raise an error:
    #   error: Argument of type "Literal[0]" cannot be assigned to parameter "name" of type "str" in function "__getitem__"
    (ext_mod_ep,) = eps
    ext_mod = ext_mod_ep.load()
    assert isinstance(ext_mod, ModuleType)

    return ext_mod


def _load_pytauri_mod(ext_mod: ModuleType) -> ModuleType:
    try:
        pytauri_mod = ext_mod.pytauri
    except AttributeError as e:
        raise RuntimeError(
            "Submodule `pytauri` is not found in the extension module"
        ) from e

    assert isinstance(pytauri_mod, ModuleType)
    return pytauri_mod


EXT_MOD = _load_ext_mod()

_pytauri_mod = _load_pytauri_mod(EXT_MOD)


_RawHandlerArgType = bytearray
# from `Vec<u8>`. See https://pyo3.rs/v0.22.2/conversions/tables#argument-types
_RawHandlerReturnType = Union[bytes, bytearray]


class _RawHandlerType(Protocol):
    def __call__(self, arg: _RawHandlerArgType, /) -> _RawHandlerReturnType: ...


class _RawInvokeHandlerType(Protocol):
    def __call__(self, func_name: str, py_func: _RawHandlerType) -> None: ...


raw_invoke_handler: _RawInvokeHandlerType = _pytauri_mod.py_invoke_handler
