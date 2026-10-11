from django.http import Http404, HttpResponsePermanentRedirect
from django.shortcuts import get_object_or_404, render
from .models import Release
from .utils import FIRST_CALENDAR_VERSION_YEAR