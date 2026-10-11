from collections.abc import AsyncIterable
import pytest
from fastapi import Depends, FastAPI, Response
from fastapi.responses import EventSourceResponse, StreamingResponse
from fastapi.testclient import TestClient
from inline_snapshot import snapshot
app = FastAPI()