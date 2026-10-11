import json
import pathlib
import pytest
from datasette.app import Datasette
from datasette.utils import StartupError
from datasette.utils.sqlite import sqlite3
from .fixtures import TestClient as _TestClient
from datasette import hookimpl