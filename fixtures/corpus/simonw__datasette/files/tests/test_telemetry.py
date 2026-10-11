import json
import sqlite3
import subprocess
import sys
import threading
import time
import pytest
import sqlite_utils
from opentelemetry import context as otel_context_api
from opentelemetry import trace as otel_trace
from opentelemetry.trace import SpanKind, StatusCode
from datasette.app import Datasette
from datasette.database import Database, QueryInterrupted
from datasette.telemetry import (
from datasette.version import __version__