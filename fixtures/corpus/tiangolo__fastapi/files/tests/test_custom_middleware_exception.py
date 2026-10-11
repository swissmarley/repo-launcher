from pathlib import Path
from fastapi import APIRouter, FastAPI, File, UploadFile
from fastapi.exceptions import HTTPException
from fastapi.testclient import TestClient
from starlette.types import ASGIApp
app = FastAPI()