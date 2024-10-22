from typing import Protocol, Any, Type, Generic, Optional, Awaitable
from contextlib import AsyncExitStack
from threading import get_ident
from collections import deque
from weakref import ref, ReferenceType
from types import TracebackType

from exceptiongroup import BaseExceptionGroup
from typing_extensions import Self, TypeVar
from anyio.from_thread import BlockingPortal
from anyio import create_task_group, CancelScope, get_cancelled_exc_class
from anyio.abc import TaskGroup

__all__ = ["RunnerBuilder"]

T = TypeVar("T", infer_variance=True)


class _PyFutureProto(Protocol, Generic[T]):
    @property
    def awaitable(self) -> Awaitable[T]: ...

    def set_result(self, result: Any, /): ...

    def set_exception(self, exception: BaseException, /): ...


class _CancelHandleProto(Protocol):
    def __call__(self) -> None: ...


class _PyRunnerProto(Protocol):
    def __call__(self, py_future: _PyFutureProto[Any], /) -> _CancelHandleProto: ...


class _RunnerProto(Protocol):
    def __new__(cls, py_runner: _PyRunnerProto, /) -> Self: ...

    def close(self) -> None: ...

    "Must call this method when `py_runner` is unavailable."


_RunnerTypeVar = TypeVar("_RunnerTypeVar", bound=_RunnerProto)


class _RunnerStack(Generic[_RunnerTypeVar]):
    def __enter__(self) -> Self:
        self._runner_stack: deque[ReferenceType[_RunnerTypeVar]] = deque()
        return self

    def push(self, runner: _RunnerTypeVar) -> None:
        self._runner_stack.append(ref(runner))

    # Implementing reference:
    # - <https://docs.python.org/3/reference/compound_stmts.html#with
    # - <https://docs.python.org/3/reference/datamodel.html#context-managers>
    #
    # NOTE: don't block too long in this method
    # NOTE: Must ensure calling `close` method of each runner
    def __exit__(
        self,
        _exc_type: Optional[type[BaseException]],
        exc: Optional[BaseException],
        _tb: Optional[TracebackType],
    ):
        excs_when_closing: list[BaseException] = []
        while self._runner_stack:
            try:
                runner = self._runner_stack.pop()()
                if runner:
                    # we require `runner` implements `close` method which
                    # wouldn't block too long
                    runner.close()
            except BaseException as e:
                excs_when_closing.append(e)

        if excs_when_closing:
            raise BaseExceptionGroup(
                "exceptions when closing runners", excs_when_closing
            ) from exc


class _PyRunner:
    def __init__(
        self,
        blocking_portal: BlockingPortal,
        task_group: TaskGroup,
        cancelled_exc_class: type[BaseException],
        event_loop_thread_id: int,
    ):
        self._blocking_portal = blocking_portal
        self._task_group = task_group
        self._cancelled_exc_class = cancelled_exc_class
        self._event_loop_thread_id = event_loop_thread_id

    def __call__(self, py_future: _PyFutureProto[Any], /) -> _CancelHandleProto:
        blocking_portal = self._blocking_portal
        task_group = self._task_group
        cancelled_exc_class = self._cancelled_exc_class
        event_loop_thread_id = self._event_loop_thread_id

        if event_loop_thread_id == get_ident():
            # DO NOT create new `TaskGroup` every time,
            # its performance is worse than `scope`;
            # use `scope` for cancellation instead.
            scope = CancelScope()

            async def _wrapped() -> None:
                is_cancelled = False
                with scope:
                    try:
                        result = await py_future.awaitable
                    except BaseException as e:
                        py_future.set_exception(e)
                        # NOTE: MUST re-raise `Cancelled` for `CancelScope`;
                        # NOTE: BUT DO NOT raise other exceptions, or it will be caught by `TaskGroup`,
                        # then `TaskGroup` will cancel all other tasks.
                        if isinstance(e, cancelled_exc_class):
                            is_cancelled = True
                            raise e
                        return
                    else:
                        py_future.set_result(result)
                        return
                # `CancelScope` will suppress (only) `Cancelled` exception,
                # so only when cancelled, this code will be executed.
                #
                # If not, it means we forget to inform rust wake up the future,
                # it will make the rust future wait forever.
                assert is_cancelled, "unreachable"

            # only the thread that created the `TaskGroup` can run following code,
            # so it's thread-safe.
            task_group.start_soon(_wrapped, name="rust_future on event loop thread")

            def cancel() -> None:
                if event_loop_thread_id == get_ident():
                    # only the thread that created the `TaskGroup` can run following code,
                    # so it's thread-safe.
                    scope.cancel()
                else:
                    # DO NOT use `blocking_portal.call`:
                    # It will block thread, but we should return as soon as possible.
                    # In fact, `blocking_portal.call` use `start_task_soon` internally (see source code).
                    blocking_portal.start_task_soon(
                        scope.cancel, name="cancel rust_future on external thread"
                    )
        else:

            async def _wrapped() -> None:
                try:
                    result = await py_future.awaitable
                except BaseException as e:
                    py_future.set_exception(e)
                    # NOTE: MUST re-raise `Cancelled` for `TaskGroup`;
                    # NOTE: BUT DO NOT raise other exceptions, or it will be caught by `TaskGroup`,
                    # then `TaskGroup` will cancel all other tasks.
                    if isinstance(e, cancelled_exc_class):
                        raise
                    return
                else:
                    py_future.set_result(result)
                    return
                # If this happens, it means we forget to inform rust wake up the future,
                # it will make the rust future wait forever.
                assert False, "unreachable"

            # NOTE: We don't care the return value (i.e, None),
            # and we should return as soon as possible so that don't block the thread.
            #
            # `start_task_soon` naturally thread-safe.
            co_future = blocking_portal.start_task_soon(
                _wrapped, name="rust_future on external thread"
            )

            def cancel() -> None:
                # whatever the thread is (event loop or external),
                # rust will use `&mut` to make sure only one thread can cancel the future at a time,
                # so this code thread-safe.
                co_future.cancel()

        return cancel


class RunnerBuilder:
    def __init__(self):
        self._exit_stack = AsyncExitStack()

    async def __aenter__(self) -> Self:
        self._event_loop_thread_id = get_ident()
        self._cancelled_exc_class = get_cancelled_exc_class()

        exit_stack = await self._exit_stack.__aenter__()
        # NOTE: keep the order of entering context managers
        # `runner_stack` must after `blocking_portal` and `task_group`
        self._blocking_portal = await exit_stack.enter_async_context(BlockingPortal())
        self._task_group = await exit_stack.enter_async_context(create_task_group())
        self._runner_stack = exit_stack.enter_context(_RunnerStack[_RunnerProto]())

        return self

    async def __aexit__(self, *exc_info: Any) -> bool:
        return await self._exit_stack.__aexit__(*exc_info)

    def build(self, runner_cls: Type[_RunnerTypeVar]) -> _RunnerTypeVar:
        py_runner = _PyRunner(
            self._blocking_portal,
            self._task_group,
            self._cancelled_exc_class,
            self._event_loop_thread_id,
        )

        runner = runner_cls(py_runner)
        self._runner_stack.push(runner)
        return runner
