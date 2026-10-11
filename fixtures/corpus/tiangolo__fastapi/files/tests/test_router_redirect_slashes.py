from fastapi import APIRouter, FastAPI
from fastapi.testclient import TestClient
    app = FastAPI()
    app = FastAPI(redirect_slashes=False)