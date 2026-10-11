import warnings
from collections.abc import Callable, Sequence
from dataclasses import dataclass
from enum import Enum
from typing import Annotated, Any, Literal
from fastapi.exceptions import FastAPIDeprecationWarning
from fastapi.openapi.models import Example
from pydantic import AliasChoices, AliasPath
from pydantic.fields import FieldInfo
from typing_extensions import deprecated
from ._compat import (
from .datastructures import _Unset
        return f"{self.__class__.__name__}({self.default})"
        return f"{self.__class__.__name__}({self.default})"