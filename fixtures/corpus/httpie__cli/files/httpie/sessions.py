import os
import re
from http.cookies import SimpleCookie
from http.cookiejar import Cookie
from pathlib import Path
from typing import Any, Dict, List, Optional, Union
from requests.auth import AuthBase
from requests.cookies import RequestsCookieJar, remove_cookie_by_name
from .context import Environment, LogLevel
from .cookies import HTTPieCookiePolicy
from .cli.dicts import HTTPHeadersDict
from .config import BaseConfigDict, DEFAULT_CONFIG_DIR
from .utils import url_as_host
from .plugins.registry import plugin_manager
from .legacy import (
                from .cli.argtypes import parse_auth