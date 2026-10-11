from collections.abc import Awaitable, Callable
from contextvars import ContextVar
from typing import Any
from fastapi import Depends, FastAPI, Request, Response
from fastapi.testclient import TestClient
app = FastAPI()