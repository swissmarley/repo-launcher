from datetime import date
from functools import reduce
from operator import and_, or_
from urllib.parse import parse_qs
from django.db import models
from .tractime import dayrange, time_property