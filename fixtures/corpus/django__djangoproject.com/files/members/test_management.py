from datetime import date, timedelta
from django.conf import settings
from django.core import mail
from django.core.management import call_command
from django.test import TestCase
from django.utils.formats import localize
from members.models import SILVER_MEMBERSHIP, CorporateMember