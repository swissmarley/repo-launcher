from datetime import date, timedelta
from django.contrib import admin
from django.test import TestCase
from .admin import CorporateMemberAdmin, StatusFilter
from .models import CorporateMember