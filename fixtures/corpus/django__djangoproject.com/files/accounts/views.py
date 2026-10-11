import hashlib
from urllib.parse import urlencode
from django.contrib.auth import logout
from django.contrib.auth.decorators import login_required
from django.contrib.auth.models import User
from django.core.cache import cache
from django.shortcuts import get_object_or_404, redirect, render
from tracdb import stats as trac_stats
from .forms import DeleteProfileForm, ProfileForm
from .models import Profile