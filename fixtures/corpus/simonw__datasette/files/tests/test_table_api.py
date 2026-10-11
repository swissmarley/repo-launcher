import json
import urllib
import pytest
from datasette.fixtures import generate_compound_rows, generate_sortable_rows
from datasette.utils import detect_json1, tilde_encode
from datasette.utils.sqlite import sqlite_version
from .fixtures import make_app_client
    from datasette import hookimpl
    from datasette.app import Datasette
        __name__ = "TestRenderCellPlugin"
    from datasette.app import Datasette
    from datasette.utils import sqlite3
    from datasette.app import Datasette