from typing import Annotated, Any
import pytest
from fastapi import Depends, FastAPI, HTTPException
from fastapi.testclient import TestClient
app = FastAPI()