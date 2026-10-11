import functools
from fastapi import FastAPI
from fastapi.testclient import TestClient
from .forward_reference_type import forwardref_method
    app = FastAPI()