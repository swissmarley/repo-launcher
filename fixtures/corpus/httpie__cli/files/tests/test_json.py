import json
import pytest
import responses
from httpie.cli.constants import PRETTY_MAP
from httpie.cli.exceptions import ParseError
from httpie.cli.nested_json import NestedJSONSyntaxError
from httpie.output.formatters.colors import ColorFormatter
from httpie.utils import JsonDictPreservingDuplicateKeys
from .fixtures import (
from .utils import DUMMY_URL, MockEnvironment, http