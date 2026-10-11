import importlib.metadata
import os
import pathlib
import re
import socket
import subprocess
import sys
import tempfile
import time
from dataclasses import dataclass
import httpx2
import pytest
import pytest_asyncio
from datasette import Event, hookimpl
    import pysqlite3 as sqlite3
    import sqlite3
from datasette.telemetry_testing import (  # noqa: F401
    from datasette.app import Datasette
    import secrets
    from datasette.app import Datasette
    from datasette.database import Database
    from .fixtures import CONFIG, METADATA, PLUGINS_DIR
    from datasette.fixtures import populate_fixture_database
    import sys
    import sys
    from datasette.default_actions import register_actions as default_register_actions
    from datasette.plugins import pm
    __name__ = "TrackEventPlugin"
    from datasette.plugins import pm
from .fixtures import (  # noqa: F401