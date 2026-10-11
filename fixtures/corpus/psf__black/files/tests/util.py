import argparse
import functools
import os
import shlex
import sys
import unittest
from collections.abc import Collection, Iterator
from contextlib import contextmanager
from dataclasses import dataclass, field, replace
from functools import partial
from pathlib import Path
from typing import Any
import black
from black.const import DEFAULT_LINE_LENGTH
from black.debug import DebugVisitor
from black.mode import TargetVersion
from black.output import diff, err, out
from black.ranges import parse_line_ranges
from . import conftest