import json
from typing import Annotated
from fastapi import APIRouter, Depends, FastAPI, WebSocket
from fastapi.testclient import TestClient
app = FastAPI(dependencies=[create_dependency("app")])