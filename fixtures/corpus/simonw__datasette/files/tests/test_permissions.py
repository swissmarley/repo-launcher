import collections
import copy
import json
import re
import time
import urllib
from pprint import pprint
import pytest
import pytest_asyncio
from asgiref.sync import async_to_sync
from bs4 import BeautifulSoup as Soup
from click.testing import CliRunner
from datasette.app import Datasette
from datasette.cli import cli
from datasette.default_permissions import restrictions_allow_action
from datasette.utils import UNSTABLE_API_MESSAGE
from .fixtures import assert_permissions_checked, make_app_client
        from datasette.resources import DatabaseResource, QueryResource, TableResource
    from datasette import hookimpl
    from datasette.permissions import PermissionSQL
    from datasette.resources import QueryResource
        __name__ = "ActorSpecificQueryPermissionPlugin"
    from datasette.resources import TableResource
    from datasette.app import Datasette
    from datasette.resources import TableResource
    from datasette.resources import TableResource
    from datasette.resources import TableResource
    from datasette import hookimpl
    from datasette.permissions import PermissionSQL
    from datasette.resources import TableResource
    from datasette.utils import sqlite3 as _sqlite3