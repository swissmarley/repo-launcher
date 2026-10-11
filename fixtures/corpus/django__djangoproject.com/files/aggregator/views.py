from django.contrib import messages
from django.contrib.auth.decorators import login_required
from django.shortcuts import get_object_or_404, redirect, render
from django.utils.translation import gettext as _
from django.views.generic.list import ListView
from .forms import FeedModelForm
from .models import APPROVED_FEED, Feed, FeedItem, FeedType, LocalDjangoCommunity