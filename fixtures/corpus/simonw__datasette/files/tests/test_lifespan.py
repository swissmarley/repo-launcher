import asyncio
import contextlib
import sqlite3
import httpx2
import pytest
from datasette import hookimpl
from datasette.app import Datasette
from datasette.database import Database
from datasette.plugins import pm
        __name__ = "RaisingStartupPlugin"
        __name__ = "CountingStartupPlugin"
        __name__ = "SlowStartupPlugin"
        __name__ = "AssertStartupPlugin"
        __name__ = "ShortCircuitPlugin"