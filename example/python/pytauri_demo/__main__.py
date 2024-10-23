from codelldb import debug
from pydantic import BaseModel
from pytauri import Commands, AppHandle, Runner, build_app, RunEvent
from pytauri_plugin_notification import NotificationExt
from pyfuture import RunnerBuilder, create_runner_builder

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

            def callback(_app_handle: AppHandle, _run_event: RunEvent) -> None:
                pass

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

        def callback(_app_handle: AppHandle, _run_event: RunEvent) -> None:
            pass

        app.run(callback)


if __name__ == "__main__":
    debug()
    sync_main()  # or `async_main()`
