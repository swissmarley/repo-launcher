import json
import os
import platform
import pprint
import subprocess
import sys
from base64 import b64encode
from os.path import dirname, join
from pathlib import Path
from typing import Any, Final
import click
import urllib3
from packaging.version import Version
    import diff_shades  # type: ignore[import-not-found]
if __name__ == "__main__":