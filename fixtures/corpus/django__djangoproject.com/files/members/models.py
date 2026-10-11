from collections import defaultdict
from django.core import signing
from django.db import models
from django.db.models.signals import post_save
from django.dispatch import receiver
from django.utils.safestring import mark_safe
from django.utils.translation import gettext_lazy as _
from django.views.generic.dates import timezone_today
from django_hosts import reverse
from sorl.thumbnail import ImageField
from djangoproject.thumbnails import LogoThumbnailMixin