import json
from typing import AsyncIterable, Iterable  # noqa: UP035 to test coverage
from fastapi import APIRouter, FastAPI
from fastapi.testclient import TestClient
from pydantic import BaseModel
app = FastAPI()