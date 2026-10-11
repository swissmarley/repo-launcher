from base64 import b64encode
from fastapi import FastAPI, Security
from fastapi.security import HTTPBasic, HTTPBasicCredentials
from fastapi.testclient import TestClient
from inline_snapshot import snapshot
app = FastAPI()