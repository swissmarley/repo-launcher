from django.contrib import admin
from django.db.models import Sum
from django.utils.translation import gettext_lazy as _
from sorl.thumbnail.admin import AdminImageMixin
from .admin_views import download_donor_report
from .models import (