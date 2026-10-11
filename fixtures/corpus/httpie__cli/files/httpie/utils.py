import os
import base64
import json
import mimetypes
import re
import sys
import time
import tempfile
import sysconfig
from collections import OrderedDict
from contextlib import contextmanager
from http.cookiejar import parse_ns_headers
from pathlib import Path
from pprint import pformat
from urllib.parse import urlsplit
from typing import Any, List, Optional, Tuple, Generator, Callable, Iterable, IO, TypeVar
import requests.auth
    from httpie.compat import (