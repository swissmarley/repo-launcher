import sys
import os
import zlib
import functools
import threading
from typing import Any, Callable, IO, Iterable, Optional, Tuple, Union, TYPE_CHECKING
from urllib.parse import urlencode
import requests
from requests.utils import super_len
    from requests_toolbelt import MultipartEncoder
from .context import Environment
from .cli.dicts import MultipartRequestDataDict, RequestDataDict
from .compat import is_windows
    import select
        from requests_toolbelt import MultipartEncoder
    from requests_toolbelt import MultipartEncoder