import os
import sys
import unittest
import subprocess
from test.helper import is_download_test, try_rm
                sys.executable, 'yt_dlp/__main__.py',
                sys.executable, 'yt_dlp/__main__.py', '--yes-overwrites',
if __name__ == '__main__':