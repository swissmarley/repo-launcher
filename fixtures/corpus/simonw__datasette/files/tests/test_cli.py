import io
import json
import pathlib
import sys
import textwrap
from unittest import mock
import pytest
from click.testing import CliRunner
from datasette.app import SETTINGS
from datasette.cli import cli, serve
from datasette.plugins import DEFAULT_PLUGINS, pm
from datasette.utils import tilde_encode
from datasette.utils.sqlite import sqlite3
from datasette.version import __version__
from .fixtures import (
from .fixtures import (
    run_module.assert_called_once_with("pip", run_name="__main__")
    run_module.assert_called_once_with("pip", run_name="__main__")
    run_module.assert_called_once_with("pip", run_name="__main__")
    run_module.assert_called_once_with("pip", run_name="__main__")
            p for p in pm.get_plugins() if p.__name__ == "startup_error.py"