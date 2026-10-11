import re
from pathlib import Path
import pytest
import datasette.fixtures  # noqa: F401
from datasette import app, utils
from datasette.app import Datasette
from datasette.filters import Filters
    from datasette.views.table_extras import CountExtra
        with subtests.test(fn=fn.__name__):