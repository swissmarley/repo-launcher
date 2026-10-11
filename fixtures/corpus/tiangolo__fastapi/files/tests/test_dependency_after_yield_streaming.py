from collections.abc import Generator
from contextlib import contextmanager
from typing import Annotated, Any
import pytest
from fastapi import Depends, FastAPI
from fastapi.responses import StreamingResponse
from fastapi.testclient import TestClient
app = FastAPI()