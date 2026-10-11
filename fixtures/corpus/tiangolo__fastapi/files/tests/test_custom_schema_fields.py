from typing import Annotated
from fastapi import FastAPI
from fastapi.testclient import TestClient
from pydantic import BaseModel, WithJsonSchema
app = FastAPI()