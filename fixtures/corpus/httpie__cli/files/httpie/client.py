import argparse
import http.client
import json
import sys
from contextlib import contextmanager
from time import monotonic
from typing import Any, Dict, Callable, Iterable
from urllib.parse import urlparse, urlunparse
import requests
import urllib3
from urllib3.util import SKIP_HEADER, SKIPPABLE_HEADERS
from . import __version__
from .adapters import HTTPieHTTPAdapter
from .cli.constants import HTTP_OPTIONS
from .cli.dicts import HTTPHeadersDict
from .cli.nested_json import unwrap_top_level_list_if_needed
from .context import Environment
from .encoding import UTF8
from .models import RequestsMessage
from .plugins.registry import plugin_manager
from .sessions import get_httpie_session
from .ssl_ import AVAILABLE_SSL_VERSION_ARG_MAPPING, HTTPieCertificate, HTTPieHTTPSAdapter
from .uploads import (
from .utils import get_expired_cookies, repr_dict