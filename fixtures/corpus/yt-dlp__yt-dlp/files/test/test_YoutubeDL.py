import os
import sys
import unittest
from unittest.mock import patch
from yt_dlp.globals import all_plugins_loaded
import contextlib
import copy
import json
from test.helper import FakeYDL, try_rm
from yt_dlp import YoutubeDL
from yt_dlp.extractor.common import InfoExtractor
from yt_dlp.postprocessor.common import PostProcessor
from yt_dlp.utils import (
from yt_dlp.utils.traversal import traverse_obj
        from http.cookiejar import Cookie
if __name__ == '__main__':