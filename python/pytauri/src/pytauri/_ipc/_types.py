from typing import Protocol, Union, Generic

from typing_extensions import TypeVar

from pydantic import BaseModel

from pytauri.ffi import (
    _RawHandlerArgType,  # pyright: ignore[reportPrivateUsage]
    _RawHandlerReturnType,  # pyright: ignore[reportPrivateUsage]
    AppHandle,
)

__all__ = ["PyHandlerTypes", "PyHandlerArgTypeVar"]

_PyHandlerArgType = Union[_RawHandlerArgType, BaseModel]
_PyHandlerReturnType = Union[_RawHandlerReturnType, BaseModel]

PyHandlerArgTypeVar = TypeVar(
    "PyHandlerArgTypeVar", bound=_PyHandlerArgType, infer_variance=True
)


class _NamedProto(Protocol):
    __name__: str


# `0` means no KEYWORD argument
class _PyHandlerType_0(_NamedProto, Protocol, Generic[PyHandlerArgTypeVar]):
    async def __call__(self, arg: PyHandlerArgTypeVar, /) -> _PyHandlerReturnType: ...


# `a` means has `app_handle` KEYWORD argument,
# if also has `webview_window` KEYWORD argument(in future), use `_PyHandlerType_aw`;
# Because we use alphabetical order, so it is `aw` instead of `wa`.
class _PyHandlerType_a(_NamedProto, Protocol, Generic[PyHandlerArgTypeVar]):
    async def __call__(
        self, arg: PyHandlerArgTypeVar, /, *, app_handle: AppHandle
    ) -> _PyHandlerReturnType: ...


PyHandlerTypes = Union[
    _PyHandlerType_0[PyHandlerArgTypeVar], _PyHandlerType_a[PyHandlerArgTypeVar]
]
