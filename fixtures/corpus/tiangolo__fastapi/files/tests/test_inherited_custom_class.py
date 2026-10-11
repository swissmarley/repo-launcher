import uuid
import pytest
from fastapi import FastAPI
from fastapi.testclient import TestClient
from pydantic import BaseModel
    from pydantic import field_serializer
    app = FastAPI()