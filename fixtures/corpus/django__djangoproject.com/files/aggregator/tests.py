import datetime
from operator import attrgetter
import requests_mock
from django.conf import settings
from django.contrib.auth.models import Group, User
from django.core import mail
from django.core.management import call_command
from django.test import SimpleTestCase, TestCase
from django.urls import reverse
from docs.models import DocumentRelease
from releases.models import Release
from . import feeds, models
from .forms import FeedModelForm