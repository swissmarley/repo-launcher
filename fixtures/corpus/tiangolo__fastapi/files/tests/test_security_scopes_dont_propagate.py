from typing import Annotated, Any
from fastapi import FastAPI, Security
from fastapi.security import SecurityScopes
from fastapi.testclient import TestClient
app = FastAPI()