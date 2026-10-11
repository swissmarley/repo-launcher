from __future__ import annotations
from dataclasses import dataclass
from typing import TYPE_CHECKING
from .permissions import Resource
from .resources import DatabaseResource, TableResource
from .utils import named_parameters, sqlite3
from .utils.asgi import Forbidden
from .utils.sql_analysis import Operation, SQLAnalysis
    from .app import Datasette