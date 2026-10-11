import hashlib
import json
import os
import pathlib
import tempfile
from unittest.mock import patch
import pytest
from datasette import utils
from datasette.app import Datasette
from datasette.utils.asgi import Request
from datasette.utils.sqlite import (