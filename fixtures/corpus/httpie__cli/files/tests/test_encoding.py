import pytest
import responses
from charset_normalizer.constant import TOO_SMALL_SEQUENCE
from httpie.cli.constants import PRETTY_MAP
from httpie.encoding import UTF8
from .utils import http, HTTP_OK, DUMMY_URL, MockEnvironment
from .fixtures import UNICODE