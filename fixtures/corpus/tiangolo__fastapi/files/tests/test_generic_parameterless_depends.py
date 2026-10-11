from typing import Annotated, TypeVar
from fastapi import Depends, FastAPI
from fastapi.testclient import TestClient
from inline_snapshot import snapshot
app = FastAPI()
    return {"cls": dep.__class__.__name__}
    return {"cls": dep.__class__.__name__}