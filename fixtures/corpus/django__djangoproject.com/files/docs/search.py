from urllib.parse import urlparse
from django.contrib.postgres.search import SearchVector
from django.db.models import TextChoices
from django.db.models.fields.json import KeyTextTransform
from django.test import Client
from django.utils.translation import gettext_lazy as _