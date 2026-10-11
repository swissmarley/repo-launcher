from fastapi import Depends, FastAPI
from fastapi.testclient import TestClient
from inline_snapshot import snapshot
app = FastAPI()