import datetime as dt
import os
import sys
import unittest
import struct
from yt_dlp import compat
from yt_dlp.compat import urllib  # isort: split
from yt_dlp.compat import compat_etree_fromstring, compat_expanduser, compat_datetime_from_timestamp
from yt_dlp.compat.urllib.request import getproxies
if __name__ == '__main__':