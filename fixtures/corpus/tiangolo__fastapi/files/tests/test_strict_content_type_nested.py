from fastapi import APIRouter, FastAPI
from fastapi.testclient import TestClient
app_nested = FastAPI(strict_content_type=False)  # lax app
app_mixed = FastAPI(strict_content_type=True)