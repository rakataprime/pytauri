import sys
import logging

from codelldb import debug
from pydantic import BaseModel
from pytauri import Commands, AppHandle, Runner, build_app, RunEvent, RunEventEnum
from pytauri_plugin_notification import NotificationExt
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
    notification.builder().title("Greeting").body(f"Hello, {person.name}!").show()

    return Greeting(message=f"Hello, {person.name}!")


def async_main() -> None:
    # or `trio` or `anyio`
    import asyncio

    async def _async_main():
        async with RunnerBuilder() as runner_builder:
            app = build_app(runner_builder.build(Runner), commands)

            if sys.version_info >= (3, 10):

                def callback(_app_handle: AppHandle, run_event: RunEvent) -> None:
                    # pyright didn't ignore this deadcode in py39 automatically,
                    # so we have to do it manually
                    match run_event.match():  # pyright: ignore
                        case RunEventEnum.ExitRequested(code=code):
                            logger.info(f"ExitRequested: {code}")
                        case event:
                            logger.debug(event)
            else:

                def callback(_app_handle: AppHandle, run_event: RunEvent) -> None:
                    run_event_enum = run_event.match()

                    if isinstance(run_event_enum, RunEventEnum.ExitRequested):
                        logger.info(f"ExitRequested: {run_event_enum.code}")
                    else:
                        logger.debug(run_event_enum)

            while True:
                app.run_iteration(callback)
                # NOTE: The smaller the value, the higher the refresh rate,
                # but the higher the performance cost
                await asyncio.sleep(0.0001)

    asyncio.run(_async_main())


def sync_main() -> None:
    backend = "asyncio"  # or `trio`
    with create_runner_builder(backend) as runner_builder:
        app = build_app(runner_builder.build(Runner), commands)

        if sys.version_info >= (3, 10):

            def callback(_app_handle: AppHandle, run_event: RunEvent) -> None:
                # pyright didn't ignore this deadcode in py39 automatically,
                # so we have to do it manually
                match run_event.match():  # pyright: ignore
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
                run_event_enum = run_event.match()

                if isinstance(run_event_enum, RunEventEnum.Ready):
                    logger.info("Ready")
                elif isinstance(run_event_enum, RunEventEnum.ExitRequested):
                    logger.info(f"ExitRequested: {run_event_enum.code}")
                elif isinstance(run_event_enum, RunEventEnum.Exit):
                    logger.info("Exit")
                else:
                    logger.debug(run_event_enum)

        app.run(callback)


if __name__ == "__main__":
    logging.basicConfig(level=logging.INFO)

    debug()
    sync_main()  # or `async_main()`
