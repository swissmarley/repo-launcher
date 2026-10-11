import pytest
from fastapi import Cookie, FastAPI, Header, Path, Query
from fastapi.testclient import TestClient
from inline_snapshot import snapshot
app = FastAPI()