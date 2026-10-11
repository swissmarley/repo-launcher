import datetime
import json
from operator import attrgetter
from unittest import mock
import requests_mock
from django.core import management
from django.http import Http404
from django.test import RequestFactory, TestCase
from django_hosts.resolvers import reverse
from djangoproject.tests import ReleaseMixin
from tracdb.models import Ticket
from tracdb.testutils import TracDBCreateDatabaseMixin
from tracdb.tractime import datetime_to_timestamp
from .models import (
from .views import index, metric_detail, metric_json