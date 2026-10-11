import datetime
import operator
from django.core.cache import cache
from django.forms.models import model_to_dict
from django.http.response import Http404, JsonResponse
from django.shortcuts import render
from django.utils.translation import gettext as _
from .models import Metric
from .utils import generation_key