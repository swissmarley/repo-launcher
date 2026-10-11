from __future__ import annotations
import os
import sys
import collections.abc
import contextlib
import dataclasses
import hashlib
import io
import itertools
import json
import pathlib
import re
import typing
import urllib.parse
import zipfile
from devscripts.tomlparse import parse_toml
from devscripts.utils import (
                f'expected {"/".join(t.__name__ for t in SUPPORTED_TYPES)} value, '
                f'got {type(value).__name__}')
    import argparse
if __name__ == '__main__':