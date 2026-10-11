from fastapi import APIRouter, FastAPI
from fastapi.testclient import TestClient
from inline_snapshot import snapshot
from pydantic import BaseModel, HttpUrl
from starlette.responses import JSONResponse
app = FastAPI()