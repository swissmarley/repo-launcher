import json
import re
from html import unescape
import pytest
from bs4 import BeautifulSoup as Soup
from datasette.app import Datasette
from datasette.resources import DatabaseResource, QueryResource
from datasette.stored_queries import StoredQuery, StoredQueryPage
from datasette.utils import UNSTABLE_API_MESSAGE
from datasette.utils.asgi import Forbidden
from datasette.utils.sqlite import sqlite3, supports_returning