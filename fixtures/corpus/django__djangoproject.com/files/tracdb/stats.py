import operator
from collections import OrderedDict, namedtuple
from django.conf import settings
from django.utils.translation import gettext_lazy as _
from .models import Revision, Ticket, TicketChange