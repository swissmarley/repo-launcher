import pytest
from dirty_equals import IsOneOf
from fastapi import FastAPI
from fastapi.testclient import TestClient
from inline_snapshot import snapshot
from pydantic import BaseModel
from tests.utils import needs_py310
    from enum import Enum
    app = FastAPI()