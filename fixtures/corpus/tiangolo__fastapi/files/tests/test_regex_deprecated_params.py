from typing import Annotated
import pytest
from fastapi import FastAPI, Query
from fastapi.exceptions import FastAPIDeprecationWarning
from fastapi.testclient import TestClient
from inline_snapshot import snapshot
from .utils import needs_py310
    app = FastAPI()