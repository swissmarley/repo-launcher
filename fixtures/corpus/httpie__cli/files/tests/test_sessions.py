import json
import os
import shutil
from contextlib import contextmanager
from datetime import datetime
from unittest import mock
from pathlib import Path
from typing import Iterator
import pytest
from .fixtures import FILE_PATH_ARG, UNICODE
from httpie.context import Environment
from httpie.encoding import UTF8
from httpie.plugins import AuthPlugin
from httpie.plugins.builtin import HTTPBasicAuth
from httpie.plugins.registry import plugin_manager
from httpie.sessions import Session
from httpie.utils import get_expired_cookies
from .test_auth_plugins import basic_auth
from .utils import DUMMY_HOST, HTTP_OK, MockEnvironment, http, mk_config_dir
from base64 import b64encode