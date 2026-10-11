import os
import sys
import contextlib
import datetime as dt
import email.message
import io
import itertools
import json
import ntpath
import pickle
import subprocess
import unittest
import unittest.mock
import warnings
import xml.etree.ElementTree
from yt_dlp.compat import (
from yt_dlp.utils import (
from yt_dlp.utils._utils import (
from yt_dlp.utils.networking import (
if __name__ == '__main__':