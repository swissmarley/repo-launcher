from typing import Any
from fastapi import APIRouter, FastAPI
from fastapi.responses import HTMLResponse, JSONResponse, PlainTextResponse
from fastapi.testclient import TestClient
from tests.utils import needs_orjson
        import orjson  # ty: ignore[unresolved-import]
app = FastAPI(default_response_class=ORJSONResponse)