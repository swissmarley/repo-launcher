import pytest
from fastapi import FastAPI
from fastapi.exceptions import FastAPIError, ResponseValidationError
from fastapi.responses import JSONResponse, Response
from fastapi.testclient import TestClient
from inline_snapshot import snapshot
from pydantic import BaseModel
app = FastAPI()
    app = FastAPI()