import os
import sys
import tempfile
from http import HTTPStatus
from io import StringIO
from unittest.mock import patch
from django.conf import settings
from django.contrib.staticfiles.testing import StaticLiveServerTestCase
from django.core.management import call_command
from django.test import TestCase, override_settings
from django.urls import NoReverseMatch, get_resolver
from django.utils.translation import activate, gettext as _
from django_hosts.resolvers import reverse
from django_recaptcha.client import RecaptchaResponse
from playwright.sync_api import expect, sync_playwright
from djangoproject.test_runner import selected_browsers
from docs.models import DocumentRelease, Release
            setattr(module, subclass.__name__, subclass)
        cls.browser = getattr(cls.playwright, cls.browser_name).launch()