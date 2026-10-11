from fastapi import Depends, FastAPI, Header, status
from fastapi.testclient import TestClient
from inline_snapshot import snapshot
app = FastAPI()