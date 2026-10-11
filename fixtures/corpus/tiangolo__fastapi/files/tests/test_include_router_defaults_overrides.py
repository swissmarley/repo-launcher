import warnings
import pytest
from fastapi import APIRouter, Depends, FastAPI, Response
from fastapi.responses import JSONResponse
from fastapi.testclient import TestClient
from inline_snapshot import snapshot
app = FastAPI(