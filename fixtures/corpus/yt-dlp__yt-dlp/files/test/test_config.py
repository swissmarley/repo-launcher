import os
import sys
import unittest
import unittest.mock
import contextlib
import itertools
from pathlib import Path
from yt_dlp.compat import compat_expanduser
from yt_dlp.options import create_parser, parseOpts
from yt_dlp.utils import Config, get_executable_path
if __name__ == '__main__':