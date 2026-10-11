from __future__ import annotations
import uuid
from dataclasses import dataclass, field
from dirty_equals import IsUUID
from fastapi import FastAPI
from fastapi.testclient import TestClient
from inline_snapshot import snapshot
app = FastAPI()