import re
from collections.abc import Sequence
from itertools import islice
from pathlib import Path
import pytest
from black.mode import UNSTABLE_FEATURES, Preview