from typing import (
    TYPE_CHECKING,
    Any,
    Generic,
    Optional,
    Union,
    final,
)

from typing_extensions import ReadOnly, TypedDict, TypeVar

from pytauri.ffi._ext_mod import pytauri_mod

__all__ = ["ArgumentsType", "Invoke", "InvokeResolver", "ParametersType"]

_ipc_mod = pytauri_mod.ipc

if TYPE_CHECKING:
    from pytauri.ffi.lib import AppHandle


class ParametersType(TypedDict, total=False):
    body: ReadOnly[Any]
    app_handle: ReadOnly[Any]


class ArgumentsType(TypedDict, total=False):
    body: bytearray
    app_handle: "AppHandle"


_ArgumentsTypeVar = TypeVar("_ArgumentsTypeVar", default=dict[str, Any])


if TYPE_CHECKING:

    @final
    class Invoke:
        @property
        def command(self) -> str: ...

        def bind_to(
            self, parameters: ParametersType
        ) -> Optional["InvokeResolver[_ArgumentsTypeVar]"]: ...

        def resolve(self, value: Union[bytearray, bytes]) -> None: ...

        def reject(self, value: str) -> None: ...

    @final
    class InvokeResolver(Generic[_ArgumentsTypeVar]):
        @property
        def arguments(self) -> _ArgumentsTypeVar: ...

        def resolve(self, value: Union[bytearray, bytes]) -> None: ...

        def reject(self, value: str) -> None: ...


else:
    Invoke = _ipc_mod.Invoke

    class InvokeResolver(_ipc_mod.InvokeResolver, Generic[_ArgumentsTypeVar]): ...
