import asyncio
import threading
import weakref
import pytest
from datasette import telemetry
from datasette.app import Datasette
from datasette.database import Database
from datasette.utils.sqlite import sqlite3
        from datasette.database import QueryInterrupted
    import gc
    import weakref
    from datasette.telemetry_registry import METRICS