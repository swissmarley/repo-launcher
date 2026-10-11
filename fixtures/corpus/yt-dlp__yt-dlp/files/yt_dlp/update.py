from __future__ import annotations
import atexit
import contextlib
import datetime as dt
import functools
import hashlib
import json
import os
import platform
import re
import subprocess
import sys
from dataclasses import dataclass
from zipimport import zipimporter
from .networking import Request
from .networking.exceptions import HTTPError, network_exceptions
from .utils import (
from .version import (
    elif (os.path.basename(sys.argv[0]) in ('__main__.py', '-m')