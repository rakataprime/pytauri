from os import environ

# This is an env var that can only be used internally by pytauri to distinguish
# between different example extension modules.
environ["_PYTAURI_DIST"] = "tauri-app"

import sys
import logging
from typing import Optional

from anyio import create_task_group
from anyio.abc import TaskGroup
from anyio.from_thread import start_blocking_portal
from pydantic import BaseModel, ConfigDict, RootModel
from pydantic.alias_generators import to_camel
from pytauri import (
    AppHandle,
    BuilderArgs,
    Commands,
    builder_factory,
    context_factory,
)

# Configure logging
logging.basicConfig(
    level=logging.DEBUG,
    format='%(asctime)s - %(name)s - %(levelname)s - %(message)s',
    stream=sys.stderr
)
logger = logging.getLogger(__name__)

commands = Commands()

class _CamelModel(BaseModel):
    """Accepts camelCase js ipc arguments for snake_case python fields."""
    model_config = ConfigDict(
        alias_generator=to_camel,
    )

class Person(_CamelModel):
    name: Optional[str] = None

Greeting = RootModel[str]

@commands.command()
async def greet(
    body: Person,
    app_handle: AppHandle,
) -> Greeting:
    """Greet a person."""
    try:
        name = body.name or "World"
        return Greeting(f"Hello, {name}! You've been greeted from Python {sys.version}!")
    except Exception as e:
        logger.error("Error in greet", exc_info=True)
        return Greeting(f"Error occurred: {str(e)}")

# Anyio `TaskGroup` can only be created in async context
task_group: TaskGroup

def main() -> None:
    """Run the tauri-app."""
    global task_group
    try:
        with (
            start_blocking_portal("asyncio") as portal,
            portal.wrap_async_context_manager(portal.call(create_task_group)) as task_group,
        ):
            logger.info("Portal and task group initialized")
            app = builder_factory().build(
                BuilderArgs(
                    context=context_factory(),
                    invoke_handler=commands.generate_handler(portal),
                )
            )
            logger.info("App built successfully, starting run()")
            app.run()
    except Exception as e:
        logger.error("Error in main()", exc_info=True)
        raise 