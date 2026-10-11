import collections
import contextlib
import optparse
import os.path
import re
import shlex
import shutil
import string
import sys
from .compat import compat_expanduser
from .cookies import SUPPORTED_BROWSERS, SUPPORTED_KEYRINGS
from .downloader.external import list_external_downloaders
from .postprocessor import (
from .postprocessor.modify_chapters import DEFAULT_SPONSORBLOCK_CHAPTER_TITLE
from .update import UPDATE_SOURCES, detect_variant, is_non_updateable
from .utils import (
from .version import CHANNEL, __version__
    deprecation_warning(f'"{__name__}._hide_login_info" is deprecated and may be removed '