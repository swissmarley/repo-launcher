import decimal
import logging
import stripe
from django.conf import settings
from django.contrib import messages
from django.core.mail import send_mail
from django.forms.models import modelformset_factory
from django.http import Http404, HttpResponse, JsonResponse
from django.shortcuts import get_object_or_404, redirect, render
from django.template.loader import render_to_string
from django.urls import reverse
from django.utils.translation import gettext as _
from django.views.decorators.cache import never_cache
from django.views.decorators.csrf import csrf_exempt
from django.views.decorators.http import require_http_methods, require_POST
from contact.forms import PlanSponsorshipForm
from members.models import CorporateMember
from .forms import DjangoHeroForm, DonationForm, PaymentForm
from .models import DjangoHero, Donation, Payment, Testimonial
from .sponsor_programs import MARKETING_STATS, SPONSORSHIP_AMOUNTS
from .sponsorship import PLANS
logger = logging.getLogger(__name__)