import base64
import datetime
import importlib
import json
import os
import pathlib
import re
import textwrap
import urllib
import pytest
from bs4 import BeautifulSoup as Soup
from click.testing import CliRunner
from jinja2 import ChoiceLoader, FileSystemLoader
from datasette import cli, hookimpl
from datasette.app import Datasette
from datasette.filters import FilterArguments
from datasette.fixtures import TABLES
from datasette.permissions import Action, PermissionSQL
from datasette.plugins import DEFAULT_PLUGINS, get_plugins, pm
from datasette.resources import DatabaseResource
from datasette.utils import StartupError, await_me_maybe
from datasette.utils.sqlite import sqlite3
from .fixtures import (
from .fixtures import (
        from datasette import hookimpl
        __name__ = "AddQueriesPlugin"
        __name__ = "AddQueryPlugin"
        __name__ = "VerifyPlugin"
        __name__ = "ReturnNothingPlugin"
        __name__ = "ActorsFromIdsPlugin"
        __name__ = "DummyPlugin"
    __name__ = "SlotPlugin"
    from .conftest import TrackEventPlugin
    assert any(k.__name__ == "OneEvent" for k in datasette.event_classes)
        __name__ = "WrapWritePlugin"
    from datasette.permissions import Resource
        __name__ = "test_custom_resources_plugin"
            from datasette.permissions import PermissionSQL