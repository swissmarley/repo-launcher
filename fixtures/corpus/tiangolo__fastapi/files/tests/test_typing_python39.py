from fastapi import FastAPI
from fastapi.testclient import TestClient
from .utils import needs_py310
        app = FastAPI()