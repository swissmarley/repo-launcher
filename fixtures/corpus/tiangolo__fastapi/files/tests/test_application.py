import pytest
from fastapi.testclient import TestClient
from inline_snapshot import snapshot
from .main import app