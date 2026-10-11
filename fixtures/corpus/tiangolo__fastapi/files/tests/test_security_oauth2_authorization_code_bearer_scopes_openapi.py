from typing import Annotated
from fastapi import APIRouter, Depends, FastAPI, Security
from fastapi.security import OAuth2AuthorizationCodeBearer
from fastapi.testclient import TestClient
from inline_snapshot import snapshot
app = FastAPI(dependencies=[Depends(get_token)])