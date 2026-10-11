from fastapi import FastAPI
from fastapi.testclient import TestClient
app = FastAPI(swagger_ui_oauth2_redirect_url=None)