import sys
import collections
import getpass
import itertools
import optparse
import os
import re
import traceback
from .cookies import SUPPORTED_BROWSERS, SUPPORTED_KEYRINGS, CookieLoadError
from .downloader.external import get_external_downloader
from .extractor import list_extractor_classes
from .extractor.adobepass import MSO_INFO
from .networking.impersonate import ImpersonateTarget
from .globals import IN_CLI, plugin_dirs
from .options import parseOpts
from .plugins import load_all_plugins as _load_all_plugins
from .postprocessor import (
from .update import Updater
from .utils import (
from .utils._utils import _UnsafeExtensionError
from .utils._jsruntime import (
from .YoutubeDL import YoutubeDL
        from .extractor.generic import GenericIE
                import ctypes.wintypes
                import msvcrt
from .extractor import gen_extractors, list_extractors
from .globals import supported_js_runtimes, supported_remote_components