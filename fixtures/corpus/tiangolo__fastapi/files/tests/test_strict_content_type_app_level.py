from fastapi import FastAPI
from fastapi.testclient import TestClient
app_default = FastAPI()
app_lax = FastAPI(strict_content_type=False)