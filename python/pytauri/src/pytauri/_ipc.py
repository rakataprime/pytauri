from typing import Protocol, Union, Callable, Optional, Any, overload, cast, Generic
from typing_extensions import TypeVar
from inspect import signature
from functools import wraps

from pydantic import BaseModel

from pytauri.ffi import (
    py_invoke_handler as rusty_py_invoke_handler,
    _HandlerArgType as _RawHandlerArgType,  # pyright: ignore[reportPrivateUsage]
    _HandlerReturnType as _RawHandlerReturnType,  # pyright: ignore[reportPrivateUsage]
    _HandlerType as _RawHandlerType,  # pyright: ignore[reportPrivateUsage]
)

__all__ = ["py_invoke_handler"]


_PyHandlerArgTypeVar = TypeVar(
    "_PyHandlerArgTypeVar",
    bound=Union[_RawHandlerArgType, BaseModel],
    infer_variance=True,
)
_PyHandlerReturnType = Union[_RawHandlerReturnType, BaseModel]


class _PyHandlerType(Protocol, Generic[_PyHandlerArgTypeVar]):
    def __call__(self, arg: _PyHandlerArgTypeVar, /) -> _PyHandlerReturnType: ...


class _NamedPyHandlerType(
    Generic[_PyHandlerArgTypeVar], _PyHandlerType[_PyHandlerArgTypeVar], Protocol
):
    __name__: str


def _py_to_raw_handler_wrapper(
    raw_handler: _PyHandlerType[_PyHandlerArgTypeVar],
) -> _RawHandlerType:
    handler_signature = signature(raw_handler)
    return_annotation = handler_signature.return_annotation
    parameters = handler_signature.parameters

    serializer = None
    first_param_annotation = next(iter(parameters.values())).annotation
    if issubclass(first_param_annotation, BaseModel):
        serializer = first_param_annotation.model_validate_json

    deserializer = None
    if issubclass(return_annotation, BaseModel):
        deserializer = return_annotation.__pydantic_serializer__.to_json

    def wrapper(arg: _RawHandlerArgType, /) -> _RawHandlerReturnType:
        nonlocal serializer, deserializer
        if serializer is not None:
            arg_ = serializer(arg)
            # i don't like use unsafe `cast`, but this is correct
            raw_handler_ = cast(_PyHandlerType[BaseModel], raw_handler)
            raw_return = raw_handler_(arg_)
        else:
            arg_ = arg
            # i don't like use unsafe `cast`, but this is correct
            raw_handler_ = cast(_PyHandlerType[bytearray], raw_handler)
            raw_return = raw_handler_(arg_)

        if deserializer is not None:
            # i don't like use unsafe `cast`, but this is correct
            raw_return_ = cast(BaseModel, raw_return)
            return deserializer(raw_return_)
        else:
            # i don't like use unsafe `cast`, but this is correct
            raw_return_ = cast(_RawHandlerReturnType, raw_return)
            return raw_return_

    if serializer is not None:
        # i don't like use unsafe `cast`, but this is correct
        raw_handler_ = cast(_PyHandlerType[BaseModel], raw_handler)
        wrapper = wraps(raw_handler_)(wrapper)
    else:
        # i don't like use unsafe `cast`, but this is correct
        raw_handler_ = cast(_PyHandlerType[bytearray], raw_handler)
        wrapper = wraps(raw_handler_)(wrapper)

    return wrapper


def _py_invoke_handler_decorator(
    func: _NamedPyHandlerType[_PyHandlerArgTypeVar], /
) -> _NamedPyHandlerType[_PyHandlerArgTypeVar]:
    name = func.__name__
    raw_handler = _py_to_raw_handler_wrapper(func)
    rusty_py_invoke_handler(name, raw_handler)
    return func


_DecoratableTypeVar = TypeVar("_DecoratableTypeVar", bound=Callable[..., Any])
_DecoratorGeneric = Callable[[_DecoratableTypeVar], _DecoratableTypeVar]


class py_invoke_handler(Generic[_PyHandlerArgTypeVar]):
    """This is a class which disguises as a function.

    Note:
        Do not use this as Generic type! It maybe become a non-generic in the future.
    """

    @overload
    def __new__(
        cls, func_name: str, /
    ) -> _DecoratorGeneric[_PyHandlerType[_PyHandlerArgTypeVar]]: ...
    @overload
    def __new__(
        cls, /
    ) -> _DecoratorGeneric[_NamedPyHandlerType[_PyHandlerArgTypeVar]]: ...
    def __new__(
        cls, func_name: Optional[str] = None, /
    ) -> Union[
        _DecoratorGeneric[_PyHandlerType[_PyHandlerArgTypeVar]],
        _DecoratorGeneric[_NamedPyHandlerType[_PyHandlerArgTypeVar]],
    ]:
        if func_name is None:
            return _py_invoke_handler_decorator

        def decorator(
            func: _PyHandlerType[_PyHandlerArgTypeVar], /
        ) -> _PyHandlerType[_PyHandlerArgTypeVar]:
            raw_handler = _py_to_raw_handler_wrapper(func)
            rusty_py_invoke_handler(func_name, raw_handler)
            return func

        return decorator
