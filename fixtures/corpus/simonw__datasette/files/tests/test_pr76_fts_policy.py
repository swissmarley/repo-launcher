import uuid
import pytest
from datasette.app import Datasette
from datasette.resources import TableResource
from datasette.utils.sqlite import sqlite3, sqlite_derived_table_dependencies