import pytest
from httpie.compat import is_windows
from .utils.matching import assert_output_matches, Expect
from .utils import HTTP_OK, MockEnvironment, http