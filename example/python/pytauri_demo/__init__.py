import sys
from logging import Logger, getLogger

from anyio.from_thread import start_blocking_portal
from pydantic import BaseModel
from pytauri import (
    AppHandle,
    BuilderArgs,
    Commands,
    RunEvent,
    RunEventEnum,
    builder_factory,
    context_factory,
)
from pytauri_plugin_notification import NotificationBuilderArgs, NotificationExt

__all__ = ["commands", "main"]

logger: Logger = getLogger(__name__)

commands: Commands = Commands()


class Person(BaseModel):
    name: str


class Greeting(BaseModel):
    message: str


@commands.register()
async def greet(body: Person, app_handle: AppHandle) -> Greeting:
    notification_builder = NotificationExt.builder(app_handle)
    notification_builder.show(
        NotificationBuilderArgs(title="Greeting", body=f"Hello, {body.name}!")
    )

    return Greeting(message=f"Hello, {body.name}!")


async def _async_task() -> None:
    from anyio import sleep

    for i in range(3, 0, -1):
        print(f"task done in {i} seconds")
        await sleep(1)
    print("task done")


def main() -> None:
    backend = "asyncio"  # or `trio`
    with start_blocking_portal(backend) as portal:
        app = builder_factory().build(
            BuilderArgs(
                context=context_factory(),
                invoke_handler=commands.build_invoke_handler(portal),
            )
        )
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

        portal.start_task_soon(_async_task)
        # or `blocking_portal.call(_async_task)` running synchronously
        app.run(callback)
