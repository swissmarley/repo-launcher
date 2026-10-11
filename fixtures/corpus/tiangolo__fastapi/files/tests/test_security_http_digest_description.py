from fastapi import FastAPI, Security
from fastapi.security import HTTPAuthorizationCredentials, HTTPDigest
from fastapi.testclient import TestClient
from inline_snapshot import snapshot
app = FastAPI()