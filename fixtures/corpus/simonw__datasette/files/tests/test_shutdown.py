import asyncio
import contextlib
import logging
import pytest
from datasette import hookimpl
from datasette.app import Datasette
from datasette.plugins import pm
        __name__ = "LifecyclePlugin"
        __name__ = "CountingShutdownPlugin"
        __name__ = "RaisingShutdownPlugin"
        __name__ = "WellBehavedPlugin"
        __name__ = "SyncShutdownPlugin"
        __name__ = "StubbornTaskPlugin"