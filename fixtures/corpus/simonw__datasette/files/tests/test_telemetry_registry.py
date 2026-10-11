import copy
import io
import itertools
import pickle
import pytest
import pytest_asyncio
from opentelemetry.trace import SpanKind
from datasette import hookimpl
from datasette import telemetry_registry as reg
from datasette.app import Datasette
from datasette.database import QueryInterrupted
from datasette.telemetry_testing import assert_metrics_conform, assert_metrics_covered
from datasette.utils.sqlite import sqlite3
    __name__ = "TelemetryRegistryBoomPlugin"
    from opentelemetry.sdk.metrics.export import (