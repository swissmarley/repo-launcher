import pytest
from httpie.context import Environment
from .utils import MockEnvironment, http
from httpie.compat import is_windows