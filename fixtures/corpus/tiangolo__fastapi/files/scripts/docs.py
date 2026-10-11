import json
import logging
import os
import re
import shutil
import subprocess
from html.parser import HTMLParser
from http.server import HTTPServer, SimpleHTTPRequestHandler
from multiprocessing import Pool
from pathlib import Path
from typing import Any
import typer
import yaml
from jinja2 import Template
from ruff.__main__ import find_ruff_bin
from slugify import slugify as py_slugify
if __name__ == "__main__":