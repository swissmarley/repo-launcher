import json
import os
from pathlib import Path
from typing import Any, Dict, Union
from . import __version__
from .compat import is_windows
from .encoding import UTF8
        config_type = type(self).__name__.lower()