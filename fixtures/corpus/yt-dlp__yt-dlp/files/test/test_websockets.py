import os
import sys
import time
import pytest
from test.helper import verify_address_availability
from yt_dlp.networking.common import Features, DEFAULT_TIMEOUT
import http.client
import http.cookiejar
import http.server
import json
import random
import ssl
import threading
from yt_dlp import socks
from yt_dlp.cookies import YoutubeDLCookieJar
from yt_dlp.dependencies import websockets
from yt_dlp.networking import Request
from yt_dlp.networking.exceptions import (
from yt_dlp.utils.traversal import traverse_obj
from yt_dlp.utils.networking import HTTPHeaderDict
    import websockets.sync.server
    import websockets.sync.client
        import websockets.sync.client
        import yt_dlp.networking._websockets
        from yt_dlp.networking._websockets import WebsocketsResponseAdapter
        from yt_dlp.networking._websockets import WebsocketsResponseAdapter