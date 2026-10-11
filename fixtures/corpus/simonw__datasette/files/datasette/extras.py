import re
from dataclasses import dataclass
from enum import Enum
from typing import ClassVar
from asyncinject import Registry
from datasette.utils.asgi import BadRequest
        return cls.name or _camel_to_snake(cls.__name__)