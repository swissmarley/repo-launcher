from datetime import datetime, time
from django.contrib.syndication.views import Feed
from django.utils.timezone import make_aware
from django.utils.translation import gettext_lazy as _
from .models import Meeting