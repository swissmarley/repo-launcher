import os
import sys
import unittest
import http.server
import threading
from test.helper import FakeYDL, expect_dict, expect_value, http_server_port
from yt_dlp.compat import compat_etree_fromstring
from yt_dlp.extractor import YoutubeIE, get_info_extractor
from yt_dlp.extractor.common import InfoExtractor
from yt_dlp.utils import (
if __name__ == '__main__':