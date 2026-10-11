from enum import Enum
from fastapi import FastAPI
from fastapi.testclient import TestClient
from inline_snapshot import snapshot
from pydantic import BaseModel, Field
app = FastAPI(title="Minimal FastAPI App", version="1.0.0")