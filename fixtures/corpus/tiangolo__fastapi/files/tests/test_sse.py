import asyncio
import time
from collections.abc import AsyncIterable, Iterable
import fastapi.routing
import pytest
from fastapi import APIRouter, FastAPI
from fastapi.responses import EventSourceResponse
from fastapi.sse import ServerSentEvent, format_sse_event
from fastapi.testclient import TestClient
from pydantic import BaseModel
app = FastAPI()
keepalive_app = FastAPI()
default_app_app = FastAPI(default_response_class=EventSourceResponse)
default_parent_app = FastAPI()