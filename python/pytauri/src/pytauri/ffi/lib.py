from typing import (
    TYPE_CHECKING,
    Any,
    Callable,
    Optional,
    Protocol,
    Union,
    final,
)

from typing_extensions import Self

from pytauri.ffi._ext_mod import pytauri_mod

__all__ = [
    "App",
    "AppHandle",
    "Builder",
    "BuilderArgs",
    "Context",
    "RunEvent",
    "RunEventEnum",
    "RunEventEnumType",
    "builder_factory",
    "context_factory",
]

if TYPE_CHECKING:
    from pytauri.ffi.ipc import Invoke


class _InvokeHandlerProto(Protocol):
    def __call__(self, invoke: "Invoke", /) -> Any: ...


_AppRunCallbackType = Callable[["AppHandle", "RunEvent"], None]


if TYPE_CHECKING:
    from pyo3_utils import PyMatchRefMixin

    @final
    class App:
        """NOTE: This class is not thread-safe, and should not be shared between threads.

        - You can only use it on the thread it was created on (must be the main thread on macOS)
        - And you need to ensure it is garbage collected on the thread it was created on,
            otherwise it will cause memory leaks
        """

        def run(self, callback: Optional[_AppRunCallbackType] = None, /) -> None:
            """Consume and run this app, will run until the app is exited.

            NOTE: If `callback` is specified, it must not raise an exception,
            otherwise it is undefined behavior, and in most cases, the program will panic.
            """

        def run_iteration(
            self, callback: Optional[_AppRunCallbackType] = None, /
        ) -> None:
            """Run this app iteratively without consuming it, calling `callback` on each iteration.

            NOTE: `callback` has the same restrictions as `App.run`.

            Tip: Approximately 2ms per call in debug mode.
            """

        def cleanup_before_exit(self, /) -> None: ...

    @final
    class AppHandle: ...

    @final
    class BuilderArgs:
        def __new__(
            cls,
            /,
            *,
            context: "Context",
            invoke_handler: Optional[_InvokeHandlerProto] = None,
        ) -> Self:
            """NOTE: The implementer of `invoke_handler` must never raise an exception,
            otherwise it is considered undefined behavior. Additionally, it must not block.
            """
            ...

    @final
    class Builder:
        """NOTE: This class is not thread-safe, and should not be shared between threads.

        - And you need to ensure it is garbage collected on the thread it was created on,
            otherwise it will cause memory leaks.
        """

        def build(self, args: BuilderArgs, /) -> App: ...

    @final
    class Context: ...

    @final
    class RunEvent(PyMatchRefMixin["RunEventEnumType"]): ...

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

    def builder_factory(*args: Any, **kwargs: Any) -> Builder: ...

    def context_factory(*args: Any, **kwargs: Any) -> Context: ...


else:
    App = pytauri_mod.App
    AppHandle = pytauri_mod.AppHandle
    Builder = pytauri_mod.Builder
    BuilderArgs = pytauri_mod.BuilderArgs
    Context = pytauri_mod.Context
    RunEvent = pytauri_mod.RunEvent
    RunEventEnum = pytauri_mod.RunEventEnum
    builder_factory = pytauri_mod.builder_factory
    context_factory = pytauri_mod.context_factory


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
