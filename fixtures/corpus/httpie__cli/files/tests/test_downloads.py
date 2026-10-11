import os
import tempfile
import time
import requests
from unittest import mock
from urllib.request import urlopen
import pytest
from requests.structures import CaseInsensitiveDict
from httpie.downloads import (
from .utils import http, MockEnvironment