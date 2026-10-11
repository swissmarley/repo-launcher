import itertools
import logging
import re
from functools import lru_cache
from typing import TYPE_CHECKING, Any
import pytest
from pytest import StashKey
log = logging.getLogger(__name__)
    from _pytest.config import Config
    from _pytest.config.argparsing import Parser
    from _pytest.mark.structures import MarkDecorator
    from _pytest.nodes import Node