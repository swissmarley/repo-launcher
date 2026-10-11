import contextlib
import json
import os
import re
import shutil
import traceback
import urllib.parse
from .utils import expand_path, traverse_obj, version_tuple, write_json_file
from .version import __version__