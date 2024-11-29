from typing import TYPE_CHECKING, Optional
from types import ModuleType

from typing_extensions import Self
from pytauri import EXT_MOD, AppHandle

__all__ = [
    "NotificationBuilderArgs",
    "NotificationBuilder",
    "Notification",
    "NotificationExt",
]


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

    class NotificationBuilderArgs:
        def __new__(
            cls, /, *, title: Optional[str] = None, body: Optional[str] = None
        ) -> Self: ...

    class NotificationBuilder:
        def show(self, args: NotificationBuilderArgs, /) -> None: ...

    class Notification:
        def builder(self) -> NotificationBuilder: ...

    class NotificationExt:
        def __new__(cls, app: AppHandle) -> Self: ...
        def notification(self) -> Notification: ...

else:
    NotificationBuilderArgs = _notification_mod.NotificationBuilderArgs
    NotificationBuilder = _notification_mod.NotificationBuilder
    Notification = _notification_mod.Notification
    NotificationExt = _notification_mod.NotificationExt
