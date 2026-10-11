import io
from unittest import mock
import pytest
import httpie
import httpie.__main__
from .fixtures import FILE_CONTENT, FILE_PATH
from httpie.cli.exceptions import ParseError
from httpie.context import Environment
from httpie.encoding import UTF8
from httpie.status import ExitStatus
from .utils import HTTP_OK, MockEnvironment, StdinBytesIO, http
        assert httpie.__main__.main() == ExitStatus.ERROR.value
        assert httpie.__main__.main() == ExitStatus.ERROR_CTRL_C.value