import datetime
import logging
import feedparser
from django.conf import settings
from django.contrib.auth.models import User
from django.core.exceptions import ValidationError
from django.db import models
from django.utils.translation import gettext_lazy as _
from django_countries.fields import CountryField
from django_push.subscriber import signals as push_signals
from django_push.subscriber.models import Subscription
log = logging.getLogger(__name__)