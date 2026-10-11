from unittest import skipIf
import requests
from django.conf import settings
from django.core import mail
from django.http import HttpRequest
from django.test import Client, TestCase
from django.test.utils import override_settings
from djangoproject.tests import ReleaseMixin, patch_captcha
from .views import FoundationContactForm