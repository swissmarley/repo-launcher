import ast
import asyncio
import inspect
import io
import itertools
import logging
import multiprocessing
import os
import pickle
import re
import subprocess
import sys
import textwrap
import types
from collections.abc import Callable, Iterator, Sequence
from concurrent.futures import ThreadPoolExecutor
from contextlib import contextmanager, redirect_stderr
from dataclasses import fields, replace
from importlib.metadata import version as imp_version
from io import BytesIO
from pathlib import Path, WindowsPath
from platform import system
from tempfile import NamedTemporaryFile, TemporaryDirectory
from typing import Any, TypeVar
from unittest.mock import MagicMock, patch
import click
import pytest
from click import unstyle
from click.testing import CliRunner as _CliRunner
from packaging.version import Version
from pathspec import GitIgnoreSpec
import black
import black.files
from black import Feature, TargetVersion
from black import re_compile_maybe_verbose as compile_pattern
from black.cache import FileData, get_cache_dir, get_cache_file
from black.debug import DebugVisitor
from black.mode import Mode, Preview
from black.output import color_diff, diff
from black.parsing import ASTSafetyError, SourceASTParseError
from black.report import Report
from black.strings import lines_with_leading_tabs_expanded
from tests.util import (
        'if __name__ == "__main__":\n'
            if __name__ == "__main__":
            if __name__ == "__main__":