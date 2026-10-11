from time import monotonic
import requests
from urllib3.util import SKIP_HEADER, SKIPPABLE_HEADERS
from enum import Enum, auto
from typing import Iterable, Union, NamedTuple
from urllib.parse import urlsplit
from .cli.constants import (
from .compat import cached_property
from .utils import split_cookies, parse_content_type_header
        raise TypeError(f"Unexpected message type: {type(message).__name__}")