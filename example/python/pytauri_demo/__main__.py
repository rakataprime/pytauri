from typing import Callable

from pydantic import BaseModel
from pytauri import py_invoke_handler, AppHandle
from pytauri.debug import debug
from pytauri_plugin_notification import NotificationExt

from pytauri_demo._ext_mod import run  # pyright: ignore[reportUnknownVariableType]

run: Callable[[], None]


class Person(BaseModel):
    name: str


class Greeting(BaseModel):
    message: str


@py_invoke_handler()
def greet(person: Person, app_handle: AppHandle) -> Greeting:
    notification_ext = NotificationExt(app_handle)
    notification = notification_ext.notification()
    notification.builder().title("Greeting").body(f"Hello, {person.name}!").show()

    return Greeting(message=f"Hello, {person.name}!")


def main():
    debug()
    run()


if __name__ == "__main__":
    main()
