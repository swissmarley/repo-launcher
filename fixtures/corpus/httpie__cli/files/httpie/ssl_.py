import ssl
from typing import NamedTuple, Optional
from urllib3.util.ssl_ import (
from .adapters import HTTPAdapter
from .compat import ensure_default_certs_loaded