from anyio.from_thread import start_blocking_portal
from pytauri import (
    BuilderArgs,
    Commands,
    builder_factory,
    context_factory,
)

commands: Commands = Commands()


def main() -> None:
    with start_blocking_portal("asyncio") as portal:  # or `trio`
        app = builder_factory().build(
            BuilderArgs(
                context=context_factory(),
                # 👇
                invoke_handler=commands.generate_handler(portal),
                # 👆
            )
        )
        app.run()
