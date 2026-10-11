import copy
import hashlib
import json
import pathlib
import re
import urllib.parse
import pytest
from bs4 import BeautifulSoup as Soup
from datasette.app import Datasette
from datasette.utils import allowed_pragmas
from .fixtures import make_app_client
from .utils import assert_footer_links, inner_html