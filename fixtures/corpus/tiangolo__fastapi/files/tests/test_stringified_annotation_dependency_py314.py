from typing import TYPE_CHECKING, Annotated
from fastapi import Depends, FastAPI
from fastapi.testclient import TestClient
from .utils import needs_py314
    app = FastAPI()