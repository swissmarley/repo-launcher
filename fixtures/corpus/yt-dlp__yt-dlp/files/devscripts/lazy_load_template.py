import importlib
import random
import re
from ..utils import (
                         f'{cls.__name__} does not have attribute {name}{bug_reports_message()}\n')
            cls._real_class = getattr(importlib.import_module(cls._module), cls.__name__)