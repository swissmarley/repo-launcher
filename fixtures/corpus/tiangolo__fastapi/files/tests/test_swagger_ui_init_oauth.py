from fastapi import FastAPI
from fastapi.testclient import TestClient
app = FastAPI(swagger_ui_init_oauth=swagger_ui_init_oauth)