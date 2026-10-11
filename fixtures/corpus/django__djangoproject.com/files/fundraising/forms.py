import stripe
from django import forms
from django.conf import settings
from django.db import transaction
from django.utils.safestring import mark_safe
from django.utils.translation import gettext_lazy as _
from django_recaptcha.fields import ReCaptchaField
from django_recaptcha.widgets import ReCaptchaV3
from .models import INTERVAL_CHOICES, LEADERSHIP_LEVEL_AMOUNT, DjangoHero, Donation