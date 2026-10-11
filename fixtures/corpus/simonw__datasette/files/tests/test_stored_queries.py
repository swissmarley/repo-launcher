import json
import re
import pytest
from asgiref.sync import async_to_sync
from bs4 import BeautifulSoup as Soup
from .fixtures import make_app_client