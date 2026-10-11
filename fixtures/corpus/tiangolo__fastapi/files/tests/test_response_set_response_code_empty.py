from typing import Any
from fastapi import FastAPI, Response
from fastapi.testclient import TestClient
from inline_snapshot import snapshot
app = FastAPI()