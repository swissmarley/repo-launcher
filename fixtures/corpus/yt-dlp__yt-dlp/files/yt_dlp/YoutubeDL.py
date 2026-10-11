import collections
import contextlib
import copy
import datetime as dt
import errno
import fileinput
import functools
import http.cookiejar
import io
import itertools
import json
import locale
import operator
import os
import random
import re
import shutil
import string
import subprocess
import sys
import tempfile
import time
import tokenize
import traceback
import unicodedata
from .cache import Cache
from .compat import urllib  # isort: split
from .compat import urllib_req_to_req
from .cookies import CookieLoadError, LenientSimpleCookie, load_cookies
from .downloader import FFmpegFD, get_suitable_downloader, shorten_protocol_name
from .downloader.rtmp import rtmpdump_version
from .extractor import gen_extractor_classes, get_info_extractor, import_extractors
from .extractor.common import UnsupportedURLIE
from .extractor.openload import PhantomJSwrapper
from .globals import (
from .minicurses import format_text
from .networking import HEADRequest, Request, RequestDirector
from .networking.common import _REQUEST_HANDLERS, _RH_PREFERENCES
from .networking.exceptions import (
from .networking.impersonate import ImpersonateRequestHandler, ImpersonateTarget
from .plugins import directories as plugin_directories, load_all_plugins
from .postprocessor import (
from .postprocessor.ffmpeg import resolve_mapping as resolve_recode_mapping
from .update import (
from .utils import (
from .utils._utils import (
from .utils.networking import (
from .version import CHANNEL, ORIGIN, RELEASE_GIT_HEAD, VARIANT, __version__
    import ctypes
                                    from inside playlist/multi_video
                import pty
                                # check with type instead of pp_key, __name__, or isinstance
            ret = str(getattr(stream, 'encoding', f'missing ({type(stream).__name__})'))
        from .compat.compat_utils import get_package_info
        from .dependencies import available_dependencies
                klass.__name__ if klass.__name__ == name else f'{klass.__name__} as {name}'
                display_list.extend(f'{plugins[-1].IE_NAME.partition("+")[2]} ({parent.__name__})'