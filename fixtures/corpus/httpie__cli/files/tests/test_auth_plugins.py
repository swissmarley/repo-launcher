from unittest import mock
from httpie.cli.constants import SEPARATOR_CREDENTIALS
from httpie.plugins import AuthPlugin
from httpie.plugins.registry import plugin_manager
from .utils import http, HTTP_OK