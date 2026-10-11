import ast
import calendar
import datetime
import requests
from django.contrib.contenttypes.fields import GenericForeignKey, GenericRelation
from django.contrib.contenttypes.models import ContentType
from django.db import connections, models
from django.db.models.functions import JSONObject
from django.utils.translation import gettext_lazy as _
from django_hosts.resolvers import reverse
from tracdb.models import Ticket
from tracdb.stats import get_trac_link