from fastapi import FastAPI, Request, WebSocket
from fastapi.exceptions import (
from fastapi.testclient import TestClient
from pydantic import BaseModel
app = FastAPI()
sub_app = FastAPI()