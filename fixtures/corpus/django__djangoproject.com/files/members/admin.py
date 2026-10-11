from datetime import date, timedelta
from django.contrib import admin
from django.templatetags.static import static
from django.utils.formats import localize
from django.utils.html import format_html
from django.utils.translation import gettext_lazy as _
from members.models import CorporateMember, IndividualMember, Invoice, Team