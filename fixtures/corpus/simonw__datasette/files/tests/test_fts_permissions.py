import pytest
from datasette import hookimpl
from datasette.app import Datasette
from datasette.permissions import Action, PermissionSQL, _permission_check_cache
from datasette.resources import DatabaseResource, TableResource
from datasette.utils.sqlite import sqlite3, sqlite_derived_table_dependencies