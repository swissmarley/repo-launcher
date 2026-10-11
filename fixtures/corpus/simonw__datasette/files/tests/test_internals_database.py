import asyncio
import threading
import uuid
from types import SimpleNamespace
import pytest
import sqlite_utils
from opentelemetry import context as otel_context_api
from datasette.app import Datasette
from datasette.database import (
from datasette.utils import Column
from datasette.utils.sqlite import (