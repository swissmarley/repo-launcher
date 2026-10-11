from datetime import timedelta
from io import StringIO
from django.contrib.auth.models import User
from django.core.management import call_command
from django.db import connection
from django.test import TestCase, override_settings
from django.test.utils import CaptureQueriesContext
from django.utils import timezone
from registration.models import RegistrationProfile
from accounts.models import Profile
from checklists.models import Releaser
from foundation.models import Banner, BoardMember, Office, Term