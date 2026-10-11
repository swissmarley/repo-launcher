from pathlib import Path
from django.contrib import admin
from django.urls import reverse
from django.utils.html import escape, format_html, format_html_join
from django.utils.translation import gettext as _, gettext_lazy
from sorl.thumbnail import get_thumbnail
from .models import ContentFormat, Entry, Event, ImageUpload