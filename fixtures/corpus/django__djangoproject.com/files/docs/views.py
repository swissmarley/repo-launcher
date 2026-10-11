import datetime
import json
from django.conf import settings
from django.contrib.contenttypes.models import ContentType
from django.contrib.sitemaps.views import x_robots_tag
from django.contrib.sites.models import Site
from django.core.paginator import InvalidPage, Paginator
from django.http import Http404, JsonResponse
from django.shortcuts import redirect, render
from django.template.response import TemplateResponse
from django.utils.translation import activate, gettext_lazy as _
from django.views.decorators.cache import cache_page
from django_hosts.resolvers import reverse
from .forms import DocSearchForm
from .models import Document, DocumentRelease
from .search import START_SEL, DocumentationCategory
from .utils import get_doc_path_or_404, get_doc_root_or_404