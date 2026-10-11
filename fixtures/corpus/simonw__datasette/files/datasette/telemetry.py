import contextvars
import re
import threading
import time
import weakref
from contextlib import contextmanager
from opentelemetry import context as otel_context_api
from opentelemetry import metrics as otel_metrics
from opentelemetry import trace as otel_trace
from opentelemetry.propagate import extract
from opentelemetry.propagators.textmap import Getter
from opentelemetry.trace import Link, SpanKind, Status, StatusCode, get_current_span
from .telemetry_registry import (
from .version import __version__
    return getattr(fn, "__qualname__", type(fn).__name__)
                span.set_attribute(ERROR_TYPE, type(exception).__name__)