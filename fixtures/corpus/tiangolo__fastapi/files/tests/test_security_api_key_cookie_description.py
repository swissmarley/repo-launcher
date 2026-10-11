from fastapi import Depends, FastAPI, Security
from fastapi.security import APIKeyCookie
from fastapi.testclient import TestClient
from inline_snapshot import snapshot
from pydantic import BaseModel
app = FastAPI()