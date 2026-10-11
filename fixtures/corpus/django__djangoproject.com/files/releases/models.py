import datetime
import re
from functools import total_ordering
from pathlib import Path
from django.conf import settings
from django.core.cache import cache
from django.core.exceptions import ValidationError
from django.core.files.storage import FileSystemStorage
from django.core.validators import RegexValidator
from django.db import models
from django.utils.functional import cached_property
from django.utils.version import get_complete_version
from .utils import (