from typing import (
    Callable,
    Any,
    cast,
    Awaitable,
    Union,
    Optional,
)
from inspect import signature
from functools import wraps, partial
from collections import UserDict
from typing import NamedTuple
from logging import getLogger

from anyio.from_thread import BlockingPortal
from typing_extensions import TypeVar
from pydantic import BaseModel, ValidationError

from pytauri.ffi.lib import (
    _InvokeHandlerProto,  # pyright: ignore[reportPrivateUsage]
    AppHandle,
)
from pytauri.ffi.ipc import Invoke, InvokeResolver, ParametersType, ArgumentsType


__all__ = [
    "Commands",
    "Invoke",
    "InvokeResolver",
    "ParametersType",
    "ArgumentsType",
    "InvokeException",
]

_logger = getLogger(__name__)

_InvokeResponse = Union[bytes, bytearray]

_PyHandlerType = Callable[..., Awaitable[_InvokeResponse]]

_WrappablePyHandlerType = Callable[..., Awaitable[Union[_InvokeResponse, BaseModel]]]

_WrappablePyHandlerTypeVar = TypeVar(
    "_WrappablePyHandlerTypeVar", bound=_WrappablePyHandlerType, infer_variance=True
)

_RegisterType = Callable[[_WrappablePyHandlerTypeVar], _WrappablePyHandlerTypeVar]


class _PyInvokHandleData(NamedTuple):
    parameters: ParametersType
    handler: _PyHandlerType
    """The `handler` can receive the parameters specified by `parameters`"""


class InvokeException(Exception):
    """
    When this exception is raised in a `Command`,
    pytauri will return it to the frontend through `Invoke.reject(value)`
    and will not log the exception on the python side.
    """

    def __init__(self, value: str) -> None:
        self.value = value


class Commands(UserDict[str, _PyInvokHandleData]):
    def __init__(self) -> None:
        super().__init__()

        data = self.data

        async def _async_invoke_handler(invoke: Invoke) -> None:
            # NOTE:
            # - the implementer of this function must not raise exceptions
            # - and must ensure to fulfill `invoke/resolver`
            resolver = None
            try:
                command = invoke.command
                handler_data = data.get(command)
                if handler_data is None:
                    invoke.reject(f"no python handler `{command}` found")
                    return

                parameters = handler_data.parameters
                handler = handler_data.handler

                resolver = invoke.bind_to(parameters)
                if resolver is None:
                    # `invoke` has already been rejected
                    return

                try:
                    resp = await handler(**resolver.arguments)
                    # TODO, PERF: idk if this will block?
                except InvokeException as e:
                    resolver.reject(e.value)
                except Exception as e:
                    # # TODO: Should we return the traceback to the frontend?
                    # # It might leak information.
                    # from traceback import format_exc
                    # resolver.reject(format_exc())
                    _logger.exception(
                        f"invoke_handler {handler}: `{handler.__name__}` raised an exception",
                        exc_info=e,
                    )
                    resolver.reject(repr(e))
                else:
                    resolver.resolve(resp)

            except Exception as e:
                msg = f"{_async_invoke_handler} implementation raised an exception, please report this as a pytauri bug"

                _logger.critical(msg, exc_info=e)
                if resolver is not None:
                    resolver.reject(msg)
                else:
                    invoke.reject(msg)
                raise

        self._async_invoke_handler = _async_invoke_handler

    def build_invoke_handler(self, portal: BlockingPortal) -> _InvokeHandlerProto:
        """
        NOTE: The `BlockingPortal` must remain valid while the returned
        `invoke_handler` is being used.
        """
        async_invoke_handler = self._async_invoke_handler

        def invoke_handler(invoke: Invoke) -> None:
            # NOTE:
            # - `invoke_handler` must not raise exception
            # - must not block

            # this func will be call in extern thread, so it's ok to use `start_task_soon`
            portal.start_task_soon(async_invoke_handler, invoke)

        return invoke_handler

    @staticmethod
    def wrap_pyfunc(
        pyfunc: _WrappablePyHandlerType,
    ) -> _PyHandlerType:
        """Wrap a Callable to conform to the definition of PyHandlerType.

        Specifically:
        - If `pyfunc` has a `KEYWORD_ONLY` parameter named `body`, check if `issubclass(body, BaseModel)` is true,
          and if so, wrap it as a new function with `body: bytearray`.
        - If `pyfunc` conforms to `issubclass(return_annotation, BaseModel)`, wrap it as a new function with `return: bytes`.
        - If not, will return the original `pyfunc`.

        The wrapper will be updated using [functools.wraps], and its `__signature__` will also be updated.
        """
        serializer = None
        deserializer = None

        BODY_KEY = "body"

        sig = signature(pyfunc)
        parameters = sig.parameters
        return_annotation = sig.return_annotation

        body_param = parameters.get(BODY_KEY)
        if body_param is not None:
            if body_param.kind not in {
                body_param.KEYWORD_ONLY,
                body_param.POSITIONAL_OR_KEYWORD,
            }:
                raise ValueError(f"Expected `{BODY_KEY}` to be KEYWORD_ONLY")
            body_type = body_param.annotation
            if issubclass(body_type, BaseModel):
                serializer = body_type.model_validate_json
            else:
                if not issubclass(body_type, bytearray):
                    raise ValueError(
                        f"Expected `{BODY_KEY}` to be subclass of {BaseModel} or {bytearray}, "
                        f"got {body_type}"
                    )

        if issubclass(return_annotation, BaseModel):
            deserializer = return_annotation.__pydantic_serializer__.to_json
        else:
            if not issubclass(return_annotation, (bytes, bytearray)):
                raise ValueError(
                    f"Expected `return_annotation` to be subclass of {BaseModel}, {bytes} or {bytearray}, "
                    f"got {return_annotation}"
                )

        if not serializer and not deserializer:
            return cast(_PyHandlerType, pyfunc)  # `cast` make typing happy

        @wraps(pyfunc)
        async def wrapper(*args: Any, **kwargs: Any) -> bytes:
            nonlocal serializer, deserializer

            if serializer is not None:
                body_bytearray = kwargs[BODY_KEY]
                assert isinstance(body_bytearray, bytearray)  # PERF
                try:
                    body_model = serializer(body_bytearray)
                except ValidationError as e:
                    raise InvokeException(str(e))
                kwargs[BODY_KEY] = body_model

            resp = await pyfunc(*args, **kwargs)

            if deserializer is not None:
                assert isinstance(resp, BaseModel)  # PERF
                return deserializer(resp)
            else:
                assert isinstance(resp, bytes)  # PERF
                return resp

        new_parameters = None
        if serializer is not None:
            new_parameters = parameters.copy()
            new_parameters[BODY_KEY] = parameters[BODY_KEY].replace(
                annotation=bytearray
            )

        # see: <https://docs.python.org/3.13/library/inspect.html#inspect.signature>
        wrapper.__signature__ = (  # pyright: ignore[reportAttributeAccessIssue]
            sig.replace(
                parameters=list(new_parameters.values()),
                return_annotation=bytes,
            )
            if new_parameters
            else sig.replace(return_annotation=bytes)
        )
        return wrapper

    @staticmethod
    def parse_parameters(
        pyfunc: _PyHandlerType, /, check_signature: bool = True
    ) -> ParametersType:
        """Check if the `signature` conforms to [ArgumentsType],
        and if the return value conforms to [bytes] or [bytearray]"""
        sig = signature(pyfunc)
        parameters = sig.parameters
        if not check_signature:
            # `cast` make typing happy
            return cast(ParametersType, parameters)

        return_annotation = sig.return_annotation
        checked_parameters: ParametersType = {}

        arguments_type = {
            "body": bytearray,
            "app_handle": AppHandle,
        }

        for name, param in parameters.items():
            correct_anna = arguments_type.get(name)
            if correct_anna is None:
                raise ValueError(
                    f"Unexpected parameter `{name}`, expected one of {list(arguments_type.keys())}"
                )
            if not issubclass(param.annotation, correct_anna):
                raise ValueError(
                    f"Expected `{name}` to be subclass of `{correct_anna}`, got `{param.annotation}`"
                )
            checked_parameters[name] = param

        if not issubclass(return_annotation, (bytes, bytearray)):
            raise ValueError(
                f"Expected return_annotation to be subclass of {bytes} or {bytearray}, got `{return_annotation}`"
            )

        return checked_parameters

    def set_command(
        self,
        command: str,
        handler: _WrappablePyHandlerType,
        /,
        check_signature: bool = True,
    ) -> None:
        new_handler = self.wrap_pyfunc(handler)
        parameters = self.parse_parameters(new_handler, check_signature=check_signature)
        self.data[command] = _PyInvokHandleData(parameters, new_handler)

    def _register(
        self,
        handler: _WrappablePyHandlerTypeVar,
        /,
        *,
        command: Optional[str] = None,
    ) -> _WrappablePyHandlerTypeVar:
        command = command or handler.__name__
        if command in self.data:
            raise ValueError(
                f"Command `{command}` already exists. If it's expected, use `set_command` instead."
            )

        self.set_command(command, handler, check_signature=True)
        return handler

    def register(
        self, command: Optional[str] = None, /
    ) -> _RegisterType[_WrappablePyHandlerTypeVar]:
        if command is None:
            return self._register
        else:
            return partial(self._register, command=command)
