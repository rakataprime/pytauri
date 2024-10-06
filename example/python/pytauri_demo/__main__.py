from pydantic import BaseModel
from pytauri import py_invoke_handler
from pytauri.debug import debug

from pytauri_demo import run


class Person(BaseModel):
    name: str


class Greeting(BaseModel):
    message: str


@py_invoke_handler()
def greet(person: Person) -> Greeting:
    return Greeting(message=f"Hello, {person.name}!")


if __name__ == "__main__":
    debug()
    run()
