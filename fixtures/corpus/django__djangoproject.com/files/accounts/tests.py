import hashlib
from django.contrib.auth.models import AnonymousUser, User
from django.core import mail
from django.core.cache import cache
from django.test import TestCase, override_settings
from django_hosts.resolvers import reverse
from accounts.forms import DeleteProfileForm, RegistrationFormWithCaptcha
from djangoproject.tests import ReleaseMixin, patch_captcha
from foundation import models as foundationmodels
from tracdb.models import Revision, Ticket, TicketChange
from tracdb.testutils import TracDBCreateDatabaseMixin