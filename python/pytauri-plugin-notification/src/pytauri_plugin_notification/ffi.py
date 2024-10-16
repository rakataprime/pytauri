from typing import TYPE_CHECKING
from types import ModuleType

from typing_extensions import Self
from pytauri import EXT_MOD, AppHandle

__all__ = ["NotificationBuilder", "Notification", "NotificationExt"]


def _load_notification_mod(ext_mod: ModuleType) -> ModuleType:
    try:
        notification_mod = ext_mod.notification
    except AttributeError as e:
        raise RuntimeError(
            "Submodule `notification` is not found in the extension module"
        ) from e

    assert isinstance(notification_mod, ModuleType)
    return notification_mod


_notification_mod = _load_notification_mod(EXT_MOD)

if TYPE_CHECKING:

    class NotificationBuilder:
        def title(self, title: str) -> Self: ...
        def body(self, body: str) -> Self: ...
        def show(self) -> None: ...

    class Notification:
        def builder(self) -> NotificationBuilder: ...

    class NotificationExt:
        def __new__(cls, app: AppHandle) -> Self: ...
        def notification(self) -> Notification: ...

else:
    NotificationBuilder = _notification_mod.NotificationBuilder
    Notification = _notification_mod.Notification
    NotificationExt = _notification_mod.NotificationExt
