import os
import sys
import unittest
import http.server
import re
import threading
from test.helper import http_server_port, try_rm
from yt_dlp import YoutubeDL
from yt_dlp.downloader.http import HttpFD
from yt_dlp.utils._utils import _YDLLogger as FakeLogger
if __name__ == '__main__':