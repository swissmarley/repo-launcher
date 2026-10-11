from typing import Annotated, Any, Literal
from dirty_equals import IsOneOf
from fastapi import FastAPI
from fastapi.testclient import TestClient
from inline_snapshot import snapshot
from pydantic import BaseModel, Field
    from pydantic import Tag
    app = FastAPI()