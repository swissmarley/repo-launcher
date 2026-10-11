from fastapi import FastAPI, Request
from fastapi.testclient import TestClient
from inline_snapshot import snapshot
app = FastAPI(openapi_prefix="/api/v1")