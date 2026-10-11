import re
from collections.abc import Iterator
from dataclasses import replace
from typing import Any
from unittest.mock import patch
import pytest
import black
from black.mode import TargetVersion
from tests.util import (