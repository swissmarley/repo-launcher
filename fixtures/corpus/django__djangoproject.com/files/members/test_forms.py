from django.conf import settings
from django.core import mail
from django.test import TestCase
from .forms import CorporateMemberSignUpForm
from .models import CorporateMember
from .utils import get_temporary_image