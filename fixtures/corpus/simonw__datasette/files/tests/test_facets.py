import json
from urllib.parse import parse_qsl, urlsplit
import pytest
from datasette.app import Datasette
from datasette.database import Database
from datasette.facets import (
from datasette.utils import detect_json1
from datasette.utils.asgi import Request
from .fixtures import make_app_client