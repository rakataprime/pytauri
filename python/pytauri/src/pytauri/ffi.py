from typing import Protocol, Union, TYPE_CHECKING
from types import ModuleType

# See: <https://pypi.org/project/backports.entry-points-selectable/>
# and: <https://docs.python.org/3/library/importlib.metadata.html#entry-points>
# Deprecated: once we no longer support versions Python 3.9, we can remove this dependency.
from importlib_metadata import (
    entry_points,  # pyright: ignore[reportUnknownVariableType]
    EntryPoint,
)

__all__ = ["raw_invoke_handler", "EXT_MOD", "AppHandle"]


def _load_ext_mod() -> ModuleType:
    eps: tuple[EntryPoint, ...] = tuple(entry_points(group="pytauri", name="ext_mod"))
    if len(eps) != 1:
        msg_list: list[tuple[str, str]] = []
        for ep in eps:
            # See: <https://packaging.python.org/en/latest/specifications/core-metadata/#core-metadata>
            # for more attributes of `dist`.
            name = ep.dist.name if ep.dist else "UNKNOWN"
            ep = repr(ep)
            msg_list.append((name, ep))

        prefix = "\n    - "
        msg = prefix.join(f"{name}: {ep}" for name, ep in msg_list)
        raise RuntimeError(
            f"Exactly one `pytauri` entry point is expected, but got:{prefix}{msg}"
        )

    ext_mod = eps[0].load()
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

if TYPE_CHECKING:

    class AppHandle: ...
else:
    AppHandle = _pytauri_mod.AppHandle


_RawHandlerArgType = bytearray
# from `Vec<u8>`. See https://pyo3.rs/v0.22.2/conversions/tables#argument-types
_RawHandlerReturnType = Union[bytes, bytearray]


class _RawHandlerType(Protocol):
    def __call__(
        self, arg: _RawHandlerArgType, /, *, app_handle: AppHandle
    ) -> _RawHandlerReturnType: ...


class _RawInvokeHandlerType(Protocol):
    def __call__(self, func_name: str, py_func: _RawHandlerType) -> None: ...


raw_invoke_handler: _RawInvokeHandlerType = _pytauri_mod.py_invoke_handler
