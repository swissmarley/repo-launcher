import os
import sys
from unittest.mock import MagicMock
import pytest
from yt_dlp.networking.common import Features, DEFAULT_TIMEOUT
import gzip
import http.client
import http.cookiejar
import http.server
import io
import logging
import pathlib
import random
import ssl
import tempfile
import threading
import time
import urllib.request
import warnings
import zlib
from email.message import Message
from http.cookiejar import CookieJar
from test.helper import (
from yt_dlp.cookies import YoutubeDLCookieJar
from yt_dlp.dependencies import brotli, curl_cffi, requests, urllib3
from yt_dlp.networking import (
from yt_dlp.networking._urllib import UrllibRH
from yt_dlp.networking.exceptions import (
from yt_dlp.networking.impersonate import (
from yt_dlp.utils import YoutubeDLError
from yt_dlp.utils._utils import _YDLLogger as FakeLogger
from yt_dlp.utils.networking import HTTPHeaderDict, std_headers
        from requests.models import Response as RequestsResponse
        from urllib3.response import HTTPResponse as Urllib3Response
        from yt_dlp.networking._requests import RequestsResponseAdapter
        import curl_cffi.requests
        from yt_dlp.networking._curlcffi import CurlCFFIResponseAdapter
        import curl_cffi.requests
        from yt_dlp.networking._curlcffi import CurlCFFIResponseReader