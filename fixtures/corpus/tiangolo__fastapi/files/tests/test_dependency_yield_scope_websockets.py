from contextvars import ContextVar
from typing import Annotated, Any
import pytest
from fastapi import Depends, FastAPI, WebSocket
from fastapi.exceptions import FastAPIError
from fastapi.testclient import TestClient
app = FastAPI()