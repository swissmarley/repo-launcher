import logging
import django
from django import forms
from django.conf import settings
from django.contrib.sites.models import Site
from django.core.mail import EmailMessage
from django.urls import reverse
from django.utils.encoding import force_bytes
from django.utils.translation import gettext_lazy as _
from django_contact_form.forms import ContactForm
from django_recaptcha.fields import ReCaptchaField
from django_recaptcha.widgets import ReCaptchaV3
from pykismet3 import Akismet, AkismetServerError
from fundraising.sponsor_programs import BANNER_LEVELS
logger = logging.getLogger(__name__)