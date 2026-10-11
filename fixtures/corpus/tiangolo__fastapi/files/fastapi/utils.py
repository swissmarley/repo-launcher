import re
import warnings
from typing import (
import fastapi
from fastapi._compat import (
from fastapi.datastructures import DefaultPlaceholder, DefaultType
from fastapi.exceptions import FastAPIDeprecationWarning, PydanticV1NotSupportedError
from pydantic.fields import FieldInfo
from ._compat import v2
    from .routing import APIRoute