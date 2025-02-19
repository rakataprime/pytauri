from os import environ
import json
import sys
import traceback
import logging

# Configure logging
logging.basicConfig(
    level=logging.DEBUG,
    format='%(asctime)s - %(name)s - %(levelname)s - %(message)s',
    stream=sys.stderr
)
logger = logging.getLogger(__name__)

# This is an env var that can only be used internally by pytauri to distinguish
# between different example extension modules.
# You don't need and shouldn't set this in your own app.
# Must be set before importing any pytauri module.
environ["_PYTAURI_DIST"] = "tauri-app"

from pytauri import Commands

commands = Commands()

def _safe_greet(body_bytes: bytes) -> bytes:
    """Safe synchronous implementation that never raises exceptions."""
    try:
        if not body_bytes:
            return b'{"message": "Hello from Python!"}'
            
        data = json.loads(body_bytes)
        name = str(data.get('name', ''))
        result = {"message": f"Hello {name} from Python!"}
        return json.dumps(result).encode('utf-8')
    except Exception as e:
        logger.error("Error in _safe_greet: %s", str(e))
        logger.error("Traceback: %s", traceback.format_exc())
        return b'{"message": "Error occurred, but hello from Python!"}'

@commands.command()
def greet(body: bytes) -> bytes:
    """Command handler that never raises exceptions."""
    logger.debug("greet called with body: %s", body)
    try:
        response = _safe_greet(body)
        logger.debug("greet returning: %s", response)
        return response
    except:  # Catch absolutely everything
        logger.error("Unexpected error in greet handler", exc_info=True)
        return b'{"message": "Unexpected error, but hello from Python!"}'

def main() -> None:
    """Run the tauri-app."""
    logger.info("Starting main()")
    
    from anyio import create_task_group
    from anyio.abc import TaskGroup
    from anyio.from_thread import start_blocking_portal
    from pytauri import BuilderArgs, builder_factory, context_factory

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
        logger.error("Error in main(): %s", str(e))
        logger.error("Traceback: %s", traceback.format_exc())
        raise
