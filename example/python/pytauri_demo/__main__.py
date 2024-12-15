"""Run the pytauri-demo application."""

import logging

from codelldb import debug

from pytauri_demo import main

logging.basicConfig(level=logging.INFO)

debug()  # optional
main()
