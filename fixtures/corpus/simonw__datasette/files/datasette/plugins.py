import importlib
import importlib.metadata as importlib_metadata
import importlib.resources as importlib_resources
import os
import sys
from pprint import pprint
import pluggy
from . import hookspecs
            plugin.__name__
            if hasattr(plugin, "__name__")
            else plugin.__class__.__name__