import sys
import os
import re
from argparse import ArgumentParser
from pathlib import Path
from zipfile import ZipFile
from datetime import datetime
from io import BytesIO
from typing import List, Optional, Tuple, Union
from urllib.parse import quote
from urllib.request import urlopen, Request
from urllib.error import HTTPError, URLError
from termcolor import colored
import ssl
import shtab
import shutil
        import colorama
if __name__ == "__main__":