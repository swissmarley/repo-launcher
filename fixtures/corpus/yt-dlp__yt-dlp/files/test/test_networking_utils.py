import os
import sys
import pytest
import io
import random
import ssl
from yt_dlp.cookies import YoutubeDLCookieJar
from yt_dlp.dependencies import certifi
from yt_dlp.networking import Response
from yt_dlp.networking._helper import (
from yt_dlp.networking.exceptions import (
from yt_dlp.socks import ProxyType
from yt_dlp.utils.networking import HTTPHeaderDict, select_proxy