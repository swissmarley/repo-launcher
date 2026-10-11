from fastapi import Depends, FastAPI
from fastapi.requests import HTTPConnection
from fastapi.testclient import TestClient
from starlette.websockets import WebSocket
app = FastAPI()