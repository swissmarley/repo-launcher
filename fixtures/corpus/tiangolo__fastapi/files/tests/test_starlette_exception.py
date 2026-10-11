from fastapi import FastAPI, HTTPException
from fastapi.testclient import TestClient
from inline_snapshot import snapshot
from starlette.exceptions import HTTPException as StarletteHTTPException
app = FastAPI()