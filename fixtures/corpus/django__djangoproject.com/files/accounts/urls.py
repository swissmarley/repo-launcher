from django.urls import include, path
from registration.backends.default.views import RegistrationView
from . import views as account_views
from .forms import RegistrationFormWithCaptcha