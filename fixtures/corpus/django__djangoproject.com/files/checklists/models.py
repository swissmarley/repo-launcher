import datetime
import json
import re
from django.contrib.auth.models import User
from django.core.validators import MaxValueValidator, MinValueValidator, RegexValidator
from django.db import models
from django.db.models.functions import Cast, Substr
from django.shortcuts import reverse
from django.template.defaultfilters import urlize
from django.template.loader import render_to_string
from django.utils.crypto import get_random_string
from django.utils.functional import cached_property
from releases.models import Release
from .templatetags.checklist_extras import enumerate_items, format_releases_for_cves
            "title": self.__class__.__name__,