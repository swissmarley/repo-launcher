import datetime
import re
import shutil
import tempfile
from pathlib import Path
from django.contrib import admin
from django.core.exceptions import ValidationError
from django.core.files.base import ContentFile
from django.template.defaultfilters import date as datefilter
from django.test import SimpleTestCase, TestCase, override_settings
from django.urls import reverse
from django.utils.safestring import SafeString
from djangoproject.tests import ReleaseMixin
from members.models import MEMBERSHIP_LEVELS, PLATINUM_MEMBERSHIP, CorporateMember
from .models import Release, get_version, upload_to_artifact, upload_to_checksum
from .templatetags.date_format import isodate
from .templatetags.release_notes import get_latest_release_version, release_notes
from .utils import get_feature_version, get_version_tuple