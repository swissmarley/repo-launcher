from django.contrib.syndication.views import Feed
from django.shortcuts import get_object_or_404
from django.utils.translation import gettext_lazy as _
from django_hosts.resolvers import reverse
from .models import FeedItem, FeedType