import os
import sys
import threading
import unittest
import pytest
import abc
import contextlib
import enum
import functools
import http.server
import json
import random
import socket
import struct
import time
from socketserver import (
from test.helper import http_server_port, verify_address_availability
from yt_dlp.networking import Request
from yt_dlp.networking.exceptions import ProxyError, TransportError
from yt_dlp.socks import (
        import websockets.sync.server
        import websockets.version
if __name__ == '__main__':