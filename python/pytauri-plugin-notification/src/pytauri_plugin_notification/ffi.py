from typing import TYPE_CHECKING, Optional, final, Union
from types import ModuleType

from typing_extensions import Self
from pytauri import EXT_MOD, AppHandle, App

__all__ = [
    "NotificationBuilderArgs",
    "NotificationBuilder",
    "NotificationExt",
    "ImplNotificationExt",
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

    @final
    class NotificationBuilderArgs:
        def __new__(
            cls, /, *, title: Optional[str] = None, body: Optional[str] = None
        ) -> Self: ...

    @final
    class NotificationBuilder:
        """
        FIXME, XXX, TODO:

        See: <https://github.com/tauri-apps/tauri/issues/3700>
        On windows, we must install the package via the `.msi` or `nsis`, or `tauri-plugin-notification` will not work.

        Tracker issue: <https://github.com/tauri-apps/plugins-workspace/issues/2156>
        """

        def show(self, args: NotificationBuilderArgs, /) -> None: ...

    @final
    class NotificationExt:
        @staticmethod
        def builder(slf: "ImplNotificationExt", /) -> NotificationBuilder: ...

else:
    NotificationBuilderArgs = _notification_mod.NotificationBuilderArgs
    NotificationBuilder = _notification_mod.NotificationBuilder
    NotificationExt = _notification_mod.NotificationExt

ImplNotificationExt = Union[App, AppHandle]
