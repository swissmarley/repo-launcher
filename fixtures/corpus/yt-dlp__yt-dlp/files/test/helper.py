import errno
import hashlib
import json
import os.path
import re
import sys
import types
import yt_dlp.extractor
from yt_dlp import YoutubeDL
from yt_dlp.utils import preferredencoding, try_call, write_string, find_available_port
    import pytest
                yield field, f'expected either {list.__name__} or {dict.__name__}, got {type(got).__name__}'
            yield field, f'expected {str.__name__}, got {type(got).__name__}'
            yield field, f'expected {expected.__name__}, got {type(got).__name__}'
                return v.__name__