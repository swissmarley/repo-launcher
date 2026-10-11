import pytest
from fastapi import APIRouter, Depends, FastAPI
from fastapi.testclient import TestClient
app = FastAPI()