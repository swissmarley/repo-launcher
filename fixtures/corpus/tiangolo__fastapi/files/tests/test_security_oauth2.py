import pytest
from fastapi import Depends, FastAPI, Security
from fastapi.security import OAuth2, OAuth2PasswordRequestFormStrict
from fastapi.testclient import TestClient
from inline_snapshot import snapshot
from pydantic import BaseModel
app = FastAPI()