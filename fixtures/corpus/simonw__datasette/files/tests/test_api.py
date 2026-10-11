import pathlib
import urllib
import pytest
from datasette.app import Datasette
from datasette.plugins import DEFAULT_PLUGINS
from datasette.resources import DatabaseResource, TableResource
from datasette.utils import UNSTABLE_API_MESSAGE, escape_sqlite, tilde_encode
from datasette.utils.sqlite import sqlite_version
from datasette.version import __version__
from .fixtures import EXPECTED_PLUGINS, make_app_client
    from datasette import hookimpl
    from datasette.app import Datasette
        __name__ = "TestRenderCellPlugin"