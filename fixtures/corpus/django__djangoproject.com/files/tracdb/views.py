from django import db
from django.http import JsonResponse
from django.shortcuts import get_object_or_404, render
from django.views.decorators.http import require_http_methods
from .models import Ticket
from .tractime import timestamp_to_datetime