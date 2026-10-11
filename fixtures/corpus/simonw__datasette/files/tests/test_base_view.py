import json
import pytest
from datasette import Request, Response
from datasette.app import Datasette
from datasette.views.base import View