from typing import (
    Protocol,
    Optional,
    Union,
    TYPE_CHECKING,
    final,
    Callable,
    Generic,
    Awaitable,
    Any,
)
from types import ModuleType

from typing_extensions import Self, TypeVar

# See: <https://pypi.org/project/backports.entry-points-selectable/>
# and: <https://docs.python.org/3/library/importlib.metadata.html#entry-points>
# Deprecated: once we no longer support versions Python 3.9, we can remove this dependency.
from importlib_metadata import (
    entry_points,  # pyright: ignore[reportUnknownVariableType]
    EntryPoint,
)

__all__ = [
    "EXT_MOD",
    "AppHandle",
    "RunEvent",
    "RunEventEnum",
    "RunEventEnumType",
    "App",
    "Commands",
    "PyFuture",
    "Runner",
    "build_app",
]


def _load_ext_mod() -> ModuleType:
    eps: tuple[EntryPoint, ...] = tuple(entry_points(group="pytauri", name="ext_mod"))
    if len(eps) == 0:
        raise RuntimeError("No `pytauri` entry point is found")
    elif len(eps) > 1:
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


T = TypeVar("T", infer_variance=True)

_RawHandlerArgType = bytearray
# from `Vec<u8>`. See https://pyo3.rs/v0.22.2/conversions/tables#argument-types
_RawHandlerReturnType = Union[bytes, bytearray]


class _RawHandlerType(Protocol):
    async def __call__(
        self, arg: _RawHandlerArgType, /, *, app_handle: "AppHandle"
    ) -> _RawHandlerReturnType: ...


_AppRunCallbackType = Callable[["AppHandle", "RunEvent"], None]


class _CancelHandleProto(Protocol):
    def __call__(self) -> None: ...


class _PyRunnerProto(Protocol):
    def __call__(self, py_future: "PyFuture[Any]", /) -> _CancelHandleProto: ...


if TYPE_CHECKING:

    @final
    class AppHandle: ...

    @final
    class RunEvent:
        def match(self, /) -> "RunEventEnumType": ...

    @final
    class RunEventEnum:
        @final
        class Exit: ...

        @final
        class ExitRequested:
            code: Optional[int]

        @final
        class WindowEvent:
            label: str

        @final
        class WebviewEvent:
            label: str

        @final
        class Ready: ...

        @final
        class Resumed: ...

        @final
        class MainEventsCleared: ...

        @final
        class MenuEvent: ...

    @final
    class App:
        def run(self, callback: _AppRunCallbackType, /) -> None: ...
        def run_iteration(self, callback: _AppRunCallbackType, /) -> None:
            """Approximately 2ms per call"""

    class Commands:
        def __new__(cls) -> Self: ...
        def invoke_handler(self, func_name: str, py_func: _RawHandlerType) -> None: ...

    @final
    class PyFuture(Generic[T]):
        @property
        def awaitable(self) -> Awaitable[T]: ...

        def set_result(self, result: Any, /) -> None: ...

        def set_exception(self, exception: BaseException, /) -> None: ...

    @final
    class Runner:
        def __new__(cls, py_runner: _PyRunnerProto, /) -> Self: ...

        def close(self) -> None:
            """Must call this method when `py_runner` is unavailable."""

    def build_app(runner: Runner, commands: Commands, /, **kwargs: Any) -> App: ...


else:
    AppHandle = _pytauri_mod.AppHandle
    RunEvent = _pytauri_mod.RunEvent
    RunEventEnum = _pytauri_mod.RunEventEnum
    App = _pytauri_mod.App
    Commands = _pytauri_mod.Commands
    Runner = _pytauri_mod.Runner

    @final
    class PyFuture(_pytauri_mod.PyFuture, Generic[T]):
        pass

    build_app = _pytauri_mod.build_app

RunEventEnumType = Union[
    RunEventEnum.Exit,
    RunEventEnum.ExitRequested,
    RunEventEnum.WindowEvent,
    RunEventEnum.WebviewEvent,
    RunEventEnum.Ready,
    RunEventEnum.Resumed,
    RunEventEnum.MainEventsCleared,
    RunEventEnum.MenuEvent,
]
