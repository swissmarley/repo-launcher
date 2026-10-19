"""Pins a package that has no builds for new Pythons, to prove an older Python is tried."""

import sys

import numpy

print(f"numpy {numpy.__version__} on Python {sys.version_info.major}.{sys.version_info.minor}")
