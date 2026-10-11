import re
from django.conf import settings
from django.http import HttpResponsePermanentRedirect
from django.http.request import split_domain_port
from django.middleware.locale import LocaleMiddleware
from django.urls import Resolver404, resolve
from django.utils.functional import cached_property
from django.utils.http import is_same_domain
        from being processed by LocaleMiddleware one should simply remove this