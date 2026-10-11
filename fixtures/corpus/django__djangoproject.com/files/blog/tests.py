from contextlib import redirect_stderr
from datetime import date, timedelta
from io import StringIO
import time_machine
from django.conf import settings
from django.contrib import admin
from django.contrib.auth.models import Permission, User
from django.contrib.contenttypes.models import ContentType
from django.core.files.base import ContentFile
from django.test import TestCase
from django.test.utils import override_settings
from django.urls import reverse
from django.utils import timezone, translation
from djangoproject.tests import ReleaseMixin
from members.models import (
from .models import ContentFormat, Entry, Event, ImageUpload
from .sitemaps import WeblogSitemap
from .templatetags.weblog import render_latest_blog_entries