import xml.etree.ElementTree as etree
from urllib.parse import urlparse
from django.conf import settings
from django.core.cache import caches
from django.db import models
from django.templatetags.static import static
from django.test import RequestFactory
from django.utils import timezone
from django.utils.cache import _generate_cache_header_key
from django.utils.formats import date_format
from django.utils.html import format_html
from django.utils.translation import gettext_lazy as _
from django_hosts.resolvers import get_host, reverse, reverse_host
from docutils.core import publish_parts
from docutils.nodes import document
from docutils.writers.html4css1 import HTMLTranslator, Writer
from markdown import Markdown
from markdown.extensions.toc import TocExtension, slugify as _md_title_slugify
from markdown.treeprocessors import Treeprocessor