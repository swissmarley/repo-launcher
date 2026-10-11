from unittest import mock
import pytest
from httpie.plugins.builtin import HTTPBasicAuth
from httpie.status import ExitStatus
from httpie.utils import ExplicitNullAuth
from .utils import http, add_auth, HTTP_OK, MockEnvironment
import httpie.cli.constants
import httpie.cli.definition