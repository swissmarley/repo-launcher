import abc
import base64
import contextlib
import functools
import json
import os
import random
import ssl
import threading
from http.server import BaseHTTPRequestHandler
from socketserver import ThreadingTCPServer
import pytest
from test.helper import http_server_port, verify_address_availability
from test.test_networking import TEST_DIR
from test.test_socks import IPv6ThreadingTCPServer
from yt_dlp.dependencies import urllib3
from yt_dlp.networking import Request
from yt_dlp.networking.exceptions import HTTPError, ProxyError, SSLError
    import urllib3.util.ssltransport