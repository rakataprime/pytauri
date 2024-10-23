from typing import (
    Callable,
    Optional,
    Any,
    overload,
    cast,
    final,
)
from inspect import signature
from functools import wraps

from typing_extensions import TypeVar, TypedDict
from pydantic import BaseModel

from pytauri import ffi
from pytauri.ffi import (
    _RawHandlerArgType,  # pyright: ignore[reportPrivateUsage]
    _RawHandlerReturnType,  # pyright: ignore[reportPrivateUsage]
    _RawHandlerType,  # pyright: ignore[reportPrivateUsage]
    AppHandle,
)
from pytauri._ipc._types import (
    PyHandlerTypes,
    PyHandlerArgTypeVar,
)


__all__ = ["Commands"]


class _PyHandlerKwargs(TypedDict, total=False):
    app_handle: AppHandle


def _py_to_raw_handler_wrapper(
    raw_handler: PyHandlerTypes[PyHandlerArgTypeVar],
) -> _RawHandlerType:
    handler_signature = signature(raw_handler)
    return_annotation = handler_signature.return_annotation
    parameters = handler_signature.parameters

    serializer = None
    # the `parameters` is ordered, so we can get the first positional parameter
    first_param_annotation = next(iter(parameters.values())).annotation
    if issubclass(first_param_annotation, BaseModel):
        serializer = first_param_annotation.model_validate_json

    deserializer = None
    if issubclass(return_annotation, BaseModel):
        deserializer = return_annotation.__pydantic_serializer__.to_json

    # TODO, XXX, FIXME(typing): I don't know how to fix this typing error
    @wraps(raw_handler)  # pyright: ignore[reportArgumentType]
    async def wrapper(
        arg: _RawHandlerArgType, /, *, app_handle: AppHandle
    ) -> _RawHandlerReturnType:
        nonlocal serializer, deserializer

        # 1.1. the first positional argument
        arg_ = arg if serializer is None else serializer(arg)
        # 1.2. the keyword arguments
        kwargs_: _PyHandlerKwargs = {}
        APP_HANDLE_KEYNAME = "app_handle"
        if APP_HANDLE_KEYNAME in parameters:
            kwargs_[APP_HANDLE_KEYNAME] = app_handle
        # 1.3. bind the arguments
        bound_arguments = handler_signature.bind(arg_, **kwargs_)

        # 2. Call the raw handler
        raw_return = await raw_handler(*bound_arguments.args, **bound_arguments.kwargs)

        # 3. Process the return value
        if deserializer is not None:
            # i don't like use unsafe `cast`, but this is correct
            raw_return_ = cast(BaseModel, raw_return)
            return deserializer(raw_return_)
        else:
            # i don't like use unsafe `cast`, but this is correct
            raw_return_ = cast(_RawHandlerReturnType, raw_return)
            return raw_return_

    return wrapper


_DecoratableTypeVar = TypeVar("_DecoratableTypeVar", bound=Callable[..., Any])
_DecoratorGeneric = Callable[[_DecoratableTypeVar], _DecoratableTypeVar]


@final
class Commands(ffi.Commands):
    @overload
    def invoke_handler(
        self, /
    ) -> _DecoratorGeneric[PyHandlerTypes[PyHandlerArgTypeVar]]: ...
    @overload
    def invoke_handler(
        self, func_name: str, /
    ) -> _DecoratorGeneric[PyHandlerTypes[PyHandlerArgTypeVar]]: ...
    def invoke_handler(  # pyright: ignore[reportIncompatibleMethodOverride]
        self, func_name: Optional[str] = None, /
    ) -> _DecoratorGeneric[PyHandlerTypes[PyHandlerArgTypeVar]]:
        """
        NOTE: only accept `async foo(...) -> Foo`,
            not accept `foo(...) -> Coroutine[None, None, Foo]`
            neither `Awaitable`.

            i.e., the return anno must be directly `Foo`
        """
        if func_name is None:

            def _py_invoke_handler_decorator(
                func: PyHandlerTypes[PyHandlerArgTypeVar],
            ) -> PyHandlerTypes[PyHandlerArgTypeVar]:
                name = func.__name__
                raw_handler = _py_to_raw_handler_wrapper(func)
                # TODO, XXX, FIXME: use `super`
                ffi.Commands.invoke_handler(self, name, raw_handler)
                return func

            return _py_invoke_handler_decorator

        def _decorator(
            func: PyHandlerTypes[PyHandlerArgTypeVar], /
        ) -> PyHandlerTypes[PyHandlerArgTypeVar]:
            raw_handler = _py_to_raw_handler_wrapper(func)
            # TODO, XXX, FIXME: use `super`
            ffi.Commands.invoke_handler(self, func_name, raw_handler)
            return func

        return _decorator
