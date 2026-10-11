import argparse
import sys
import os
import warnings
from contextlib import contextmanager
from pathlib import Path
from typing import Iterator, IO, Optional, TYPE_CHECKING
from enum import Enum
    import curses
from .compat import is_windows, cached_property
from .config import DEFAULT_CONFIG_DIR, Config, ConfigFileError
from .encoding import UTF8
from .utils import repr_dict
from .output.ui.palette import GenericColor
    from rich.console import Console
        import colorama.initialise
                from colorama import AnsiToWin32
        return f'<{type(self).__name__} {self}>'
        from rich.console import Console
        from httpie.output.ui.rich_palette import _make_rich_color_theme