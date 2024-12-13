from typing import TYPE_CHECKING, Generic

from typing_extensions import TypeVar

_T = TypeVar("_T", infer_variance=True)

if TYPE_CHECKING:
    __all__ = ["PyMatchIntoMixin", "PyMatchMutMixin", "PyMatchRefMixin"]

    class PyMatchRefMixin(Generic[_T]):
        """This class is only used to provide type annotations,
        the actual implementation of the methods is handled by
        the pyo3 extension module.

        NOTE: This class can only be used when `TYPE_CHECKING`.
        """

        def match_ref(self, /) -> _T: ...

    class PyMatchMutMixin(Generic[_T]):
        """This class is only used to provide type annotations,
        the actual implementation of the methods is handled by
        the pyo3 extension module.

        NOTE: This class can only be used when `TYPE_CHECKING`.
        """

        def match_mut(self, /) -> _T: ...

    class PyMatchIntoMixin(Generic[_T]):
        """This class is only used to provide type annotations,
        the actual implementation of the methods is handled by
        the pyo3 extension module.

        NOTE: This class can only be used when `TYPE_CHECKING`.
        """

        def match_into(self, /) -> _T: ...
