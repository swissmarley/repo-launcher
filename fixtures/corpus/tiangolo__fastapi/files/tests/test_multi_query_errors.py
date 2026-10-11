from fastapi import FastAPI, Query
from fastapi.testclient import TestClient
from inline_snapshot import snapshot
app = FastAPI()