from __future__ import annotations
from typing import Annotated
from fastapi import Depends, FastAPI, Request
from fastapi.testclient import TestClient
from .utils import needs_py310
    app = FastAPI()