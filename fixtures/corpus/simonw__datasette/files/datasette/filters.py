import json
import math
from typing import ClassVar
from datasette import hookimpl
from datasette.resources import DatabaseResource, TableResource
from datasette.utils.asgi import BadRequest
from datasette.views.base import DatasetteError
from .utils import detect_json1, escape_sqlite, path_with_removed_args