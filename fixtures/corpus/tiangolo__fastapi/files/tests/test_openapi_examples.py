from fastapi import Body, Cookie, FastAPI, Header, Path, Query
from fastapi.testclient import TestClient
from inline_snapshot import snapshot
from pydantic import BaseModel
app = FastAPI()