import os
import sys
import unittest
from test.helper import get_params, is_download_test, try_rm
import yt_dlp.YoutubeDL  # isort: split
from yt_dlp.utils import DownloadError
if __name__ == '__main__':