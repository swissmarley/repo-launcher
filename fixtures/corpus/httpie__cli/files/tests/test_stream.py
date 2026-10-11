import json
import pytest
import responses
from unittest.mock import Mock
from httpie.compat import is_windows
from httpie.cli.constants import PRETTY_MAP
from httpie.output.streams import BINARY_SUPPRESSED_NOTICE
from httpie.plugins import ConverterPlugin
from httpie.plugins.registry import plugin_manager
from .utils import StdinBytesIO, http, MockEnvironment, DUMMY_URL
from .fixtures import (