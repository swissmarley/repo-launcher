from django import forms
from django.db import transaction
from django.db.models import ProtectedError
from django.utils.translation import gettext_lazy as _
from django_recaptcha.fields import ReCaptchaField
from django_recaptcha.widgets import ReCaptchaV3
from registration.forms import RegistrationFormUniqueEmail
from .models import Profile