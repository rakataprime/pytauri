import sys
import logging

from codelldb import debug
from pydantic import BaseModel
from pytauri import Commands, AppHandle, Runner, build_app, RunEvent, RunEventEnum
from pytauri_plugin_notification import NotificationExt, NotificationBuilderArgs
from pyfuture import RunnerBuilder, create_runner_builder

logger = logging.getLogger(__name__)

commands: Commands = Commands()


class Person(BaseModel):
    name: str


class Greeting(BaseModel):
    message: str


@commands.invoke_handler()
async def greet(person: Person, app_handle: AppHandle) -> Greeting:
    notification_ext = NotificationExt(app_handle)
    notification = notification_ext.notification()

    notification.builder().show(
        NotificationBuilderArgs(title="Greeting", body=f"Hello, {person.name}!")
    )

    return Greeting(message=f"Hello, {person.name}!")


async def _async_task() -> None:
    from anyio import sleep

    for i in range(3, 0, -1):
        print(f"task done in {i} seconds")
        await sleep(1)
    print("task done")


def async_main() -> None:
    """async version of `sync_main`

    WARNING:
        - This function has a higher performance cost because it needs to continuously call `app.run_iteration` in a loop,
            which requires constantly accessing local thread variables in Rust;
        - And, `app.run_iteration` will **block the python async event loop**!

        It is recommended to use `sync_main` and use `runner_builder.blocking_portal` to implement asynchronous code.

    TODO:
        In the future, we might provide a `run_iteration_unchecked` method,
        which does not check if it is called in the thread that created the App each time it is called.
        This might improve performance? idk XD
    """
    # or `trio` or `anyio`
    import asyncio

    async def _async_main():
        async with RunnerBuilder() as runner_builder:
            app = build_app(runner_builder.build(Runner), commands)

            exit_requested = False

            if sys.version_info >= (3, 10):

                def callback(_app_handle: AppHandle, run_event: RunEvent) -> None:
                    nonlocal exit_requested

                    # pyright didn't ignore this deadcode in py39 automatically,
                    # so we have to do it manually
                    match run_event.match_ref():  # pyright: ignore
                        case RunEventEnum.ExitRequested(code=code):
                            logger.info(f"ExitRequested: {code}")
                            exit_requested = True
                        case event:
                            logger.debug(event)
            else:

                def callback(_app_handle: AppHandle, run_event: RunEvent) -> None:
                    nonlocal exit_requested

                    run_event_enum = run_event.match_ref()
                    if isinstance(run_event_enum, RunEventEnum.ExitRequested):
                        logger.info(f"ExitRequested: {run_event_enum.code}")
                        exit_requested = True
                    else:
                        logger.debug(run_event_enum)

            # NOTE: `run_iteration`will block the python async event loop,
            # so this task will pause during the calling of `run_iteration`.
            asyncio.create_task(_async_task())
            # or `await _async_task()` running synchronously
            try:
                while not exit_requested:
                    app.run_iteration(callback)
                    # NOTE: The smaller the value, the higher the refresh rate,
                    # but the higher the performance cost
                    await asyncio.sleep(0.0001)
            finally:
                # necessary for `run_iteration`, but not necessary for `run`,
                # because `run` will call `cleanup_before_exit` for you
                app.cleanup_before_exit()

    asyncio.run(_async_main())


def sync_main() -> None:
    backend = "asyncio"  # or `trio`
    with create_runner_builder(backend) as runner_builder:
        app = build_app(runner_builder.build(Runner), commands)

        if sys.version_info >= (3, 10):

            def callback(_app_handle: AppHandle, run_event: RunEvent) -> None:
                # pyright didn't ignore this deadcode in py39 automatically,
                # so we have to do it manually
                match run_event.match_ref():  # pyright: ignore
                    case RunEventEnum.Ready():
                        logger.info("Ready")
                    case RunEventEnum.ExitRequested(code=code):
                        logger.info(f"ExitRequested: {code}")
                    case RunEventEnum.Exit():
                        logger.info("Exit")
                    case event:
                        logger.debug(event)

        else:

            def callback(_app_handle: AppHandle, run_event: RunEvent) -> None:
                run_event_enum = run_event.match_ref()

                if isinstance(run_event_enum, RunEventEnum.Ready):
                    logger.info("Ready")
                elif isinstance(run_event_enum, RunEventEnum.ExitRequested):
                    logger.info(f"ExitRequested: {run_event_enum.code}")
                elif isinstance(run_event_enum, RunEventEnum.Exit):
                    logger.info("Exit")
                else:
                    logger.debug(run_event_enum)

        # NOTE: This `blocking_portal` is the blocking_portal that runs the async commands `greets`
        blocking_portal = runner_builder.blocking_portal

        blocking_portal.start_task_soon(_async_task)
        # or `blocking_portal.call(_async_task)` running synchronously
        app.run(callback)


if __name__ == "__main__":
    logging.basicConfig(level=logging.INFO)

    debug()
    sync_main()  # or `async_main()`
