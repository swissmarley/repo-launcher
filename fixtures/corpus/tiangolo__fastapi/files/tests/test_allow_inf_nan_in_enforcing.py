from typing import Annotated
import pytest
from fastapi import Body, FastAPI, Query
from fastapi.testclient import TestClient
app = FastAPI()