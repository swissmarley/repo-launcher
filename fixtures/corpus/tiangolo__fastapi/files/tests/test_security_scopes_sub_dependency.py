from typing import Annotated
import pytest
from fastapi import Depends, FastAPI, Security
from fastapi.security import SecurityScopes
from fastapi.testclient import TestClient
    app = FastAPI()