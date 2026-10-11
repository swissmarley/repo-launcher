import functools
import pytest
from fastapi import (
from fastapi.middleware import Middleware
from fastapi.testclient import TestClient
app = FastAPI()
    app = app or FastAPI(**kwargs)