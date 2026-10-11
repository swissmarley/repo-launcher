import asyncio
import contextlib
import logging
import httpx2
import pytest
from datasette import hookimpl
from datasette.app import Datasette
from datasette.plugins import pm
        __name__ = "TwoTaskPlugin"
        __name__ = "PluginA"
        __name__ = "PluginB"
        __name__ = "CountingTaskPlugin"