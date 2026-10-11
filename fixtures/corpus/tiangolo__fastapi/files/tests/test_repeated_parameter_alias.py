from fastapi import FastAPI, Path, Query, status
from fastapi.testclient import TestClient
from inline_snapshot import snapshot
app = FastAPI()