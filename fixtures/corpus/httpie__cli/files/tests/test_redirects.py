import pytest
from httpie.compat import is_windows
from httpie.status import ExitStatus
from .fixtures import FILE_PATH_ARG, FILE_CONTENT
from .utils import http, HTTP_OK
from .utils.matching import assert_output_matches, Expect, ExpectSequence