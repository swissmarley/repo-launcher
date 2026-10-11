from collections.abc import AsyncIterable
import anyio
import pytest
from fastapi import FastAPI
from fastapi.responses import StreamingResponse
from starlette.types import Message, Scope
app = FastAPI()