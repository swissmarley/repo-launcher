import ssl
import pytest
import pytest_httpbin.certs
import requests.exceptions
import urllib3
from unittest import mock
from httpie.ssl_ import AVAILABLE_SSL_VERSION_ARG_MAPPING, DEFAULT_SSL_CIPHERS_STRING
from httpie.status import ExitStatus
from .utils import HTTP_OK, TESTS_ROOT, IS_PYOPENSSL, http
    import OpenSSL.SSL