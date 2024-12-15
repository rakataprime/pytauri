"""See: <https://setuptools-rust.readthedocs.io/en/latest/setuppy_tutorial.html>"""

from os import getenv

from setuptools import (
    setup,  # pyright: ignore[reportUnknownVariableType]
)
from setuptools_rust import RustExtension

########## CONFIGURATION ##########

PYTAURI_DEV = getenv("PYTAURI_DEV") == "1"
PYTAURI_STANDALONE = getenv("PYTAURI_STANDALONE") == "1"
# The last part of the name (e.g. "_ext_mod") has to match `lib.name` in Cargo.toml.
EXT_MOD = "pytauri_demo._ext_mod"


####################################


def get_features() -> list[str]:
    """Set the rust features for building the extension module."""
    # see: <https://pyo3.rs/v0.23.3/building-and-distribution.html#the-extension-module-feature>
    features = ["pyo3/extension-module"]
    if not PYTAURI_DEV:
        # This feature tells Tauri to use embedded frontend assets instead of using a frontend development server
        features.append("tauri/custom-protocol")
    return features


setup(
    # See reference for RustExtension in https://setuptools-rust.readthedocs.io/en/latest/reference.html
    rust_extensions=[
        RustExtension(
            target=EXT_MOD,
            features=get_features(),
        )
    ]
    if not PYTAURI_STANDALONE
    # `tauri-cli` will build an exe, and the extension module will be provided by the exe instead.
    else [],
    # See syntax: <https://setuptools.pypa.io/en/latest/userguide/entry_point.html#entry-points-for-plugins>
    entry_points={"pytauri": [f"ext_mod = {EXT_MOD}"]},
)
