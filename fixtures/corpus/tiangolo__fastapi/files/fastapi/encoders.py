import dataclasses
import datetime
from collections import defaultdict, deque
from collections.abc import Callable
from decimal import Decimal
from enum import Enum
from ipaddress import (
from pathlib import Path, PurePath
from re import Pattern
from types import GeneratorType
from typing import Annotated, Any
from uuid import UUID
from annotated_doc import Doc
from fastapi.exceptions import PydanticV1NotSupportedError
from fastapi.types import IncEx
from pydantic import BaseModel
from pydantic.networks import AnyUrl, NameEmail
from pydantic.types import SecretBytes, SecretStr
from pydantic_core import PydanticUndefinedType
from ._compat import (
    from pydantic.color import Color  # ty: ignore[deprecated]
    from pydantic_extra_types.color import Color as PyExtraColor