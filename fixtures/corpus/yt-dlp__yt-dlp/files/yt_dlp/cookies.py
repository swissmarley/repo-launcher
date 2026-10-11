import base64
import collections
import contextlib
import datetime as dt
import functools
import glob
import hashlib
import http.cookiejar
import http.cookies
import io
import json
import os
import re
import shutil
import struct
import subprocess
import sys
import tempfile
import time
import urllib.request
from enum import Enum, auto
from .aes import (
from .dependencies import (
from .minicurses import MultilinePrinter, QuietMultilinePrinter
from .utils import (
from .utils._utils import _YDLLogger
from .utils.networking import normalize_url
    import ctypes
    import ctypes.wintypes