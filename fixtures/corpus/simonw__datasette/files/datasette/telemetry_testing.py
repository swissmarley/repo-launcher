    from datasette.telemetry_testing import (  # noqa: F401
import subprocess
import sys
import pytest
from .telemetry_registry import (
        from opentelemetry import trace as otel_trace
        from opentelemetry.sdk.trace import TracerProvider
        from opentelemetry.sdk.trace.export import SimpleSpanProcessor
        from opentelemetry.sdk.trace.export.in_memory_span_exporter import (
        from opentelemetry import metrics as otel_metrics_api
        from opentelemetry.sdk.metrics import Counter, Histogram, MeterProvider
        from opentelemetry.sdk.metrics.export import (
        actual_data_type = type(metric.data).__name__