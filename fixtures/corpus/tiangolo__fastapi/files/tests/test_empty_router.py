import pytest
from fastapi import APIRouter, FastAPI
from fastapi.exceptions import FastAPIError
from fastapi.testclient import TestClient
app = FastAPI()