import warnings
import pytest
from fastapi import FastAPI
from fastapi.exceptions import FastAPIDeprecationWarning
from fastapi.responses import ORJSONResponse, UJSONResponse  # ty: ignore[deprecated]
from fastapi.testclient import TestClient
from pydantic import BaseModel
from tests.utils import needs_orjson, needs_ujson
        app = FastAPI(default_response_class=ORJSONResponse)  # ty: ignore[deprecated]
        app = FastAPI(default_response_class=UJSONResponse)  # ty: ignore[deprecated]