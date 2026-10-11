import argparse
from pathlib import Path
from unittest import mock
import json
import os
import io
import warnings
from urllib.request import urlopen
import pytest
import requests
import responses
from httpie.cli.argtypes import (
from httpie.cli.definition import parser
from httpie.encoding import UTF8
from httpie.output.formatters.colors import get_lexer, PIE_STYLE_NAMES, BUNDLED_STYLES
from httpie.status import ExitStatus
from .fixtures import XML_DATA_RAW, XML_DATA_FORMATTED
from .utils import COLOR, CRLF, HTTP_OK, MockEnvironment, http, DUMMY_URL, strip_colors