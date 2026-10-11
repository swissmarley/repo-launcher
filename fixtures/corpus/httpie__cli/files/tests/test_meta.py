import pytest
from httpie.models import ELAPSED_TIME_LABEL
from httpie.output.formatters.colors import PIE_STYLE_NAMES
from .utils import http, MockEnvironment, COLOR