from datetime import date, timedelta
from django.test import TestCase
from django.urls import reverse
from djangoproject.tests import ReleaseMixin
from .models import CorporateMember, IndividualMember, Team
from .utils import get_temporary_image