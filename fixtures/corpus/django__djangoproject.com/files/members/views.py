from django.core import signing
from django.http import Http404
from django.shortcuts import render
from django.urls import reverse
from django.utils.translation import gettext as _
from django.views.generic import CreateView, ListView, TemplateView, UpdateView
from django.views.generic.dates import timezone_today
from .forms import CorporateMemberSignUpForm
from .models import (