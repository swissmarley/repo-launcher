from fastapi import Depends, FastAPI, Query
from fastapi.testclient import TestClient
from inline_snapshot import snapshot
app = FastAPI()