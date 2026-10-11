import warnings
from collections import deque
from dataclasses import dataclass
from datetime import datetime, timezone
from decimal import Decimal
from enum import Enum
from math import isinf, isnan
from pathlib import PurePath, PurePosixPath, PureWindowsPath
from typing import TypedDict
import pytest
from fastapi._compat import Undefined
from fastapi.encoders import jsonable_encoder
from fastapi.exceptions import PydanticV1NotSupportedError
from pydantic import BaseModel, Field, ValidationError
    from pydantic import field_serializer
        from pydantic import v1