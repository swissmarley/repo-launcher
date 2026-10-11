from collections.abc import AsyncIterable, Iterable
import pytest
from fastapi import FastAPI
from fastapi.exceptions import ResponseValidationError
from fastapi.testclient import TestClient
from pydantic import BaseModel
app = FastAPI()