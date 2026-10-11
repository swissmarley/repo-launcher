import pytest
from opentelemetry import trace as otel_trace
from datasette import telemetry_registry as reg
from datasette.telemetry import linked_root_span_kwargs
from datasette.telemetry_testing import (
import itertools
from opentelemetry import metrics as otel_metrics_api
from datasette.telemetry_testing import (
from datasette.telemetry_testing import assert_no_forbidden_values