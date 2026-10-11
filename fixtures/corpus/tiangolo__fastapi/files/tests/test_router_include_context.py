import threading
from collections.abc import Sequence
from typing import Annotated, cast
import pytest
from fastapi import APIRouter, Body, Depends, FastAPI, Request, Security
from fastapi.exceptions import FastAPIError
from fastapi.openapi.utils import get_openapi
from fastapi.responses import HTMLResponse, JSONResponse, PlainTextResponse
from fastapi.routing import (
from fastapi.security import HTTPBearer
from fastapi.testclient import TestClient
from pydantic import BaseModel
from starlette.routing import BaseRoute, Host, Match, Mount, NoMatchFound, Route, Router
    app = FastAPI()
    app = FastAPI()
                "response_class": context.response_class.__name__,
    app = FastAPI()
    app = FastAPI()
    app = FastAPI()
    app = FastAPI()
    app = FastAPI()
    app = FastAPI()
    app = FastAPI()
    app = FastAPI()
    app = FastAPI()
    app = FastAPI()
    app = FastAPI()
    app = FastAPI()
    app = FastAPI()
    app = FastAPI(dependencies=[Depends(app_body_dependency)])
    app = FastAPI()
    app = FastAPI()
    app = FastAPI()
    app = FastAPI()
    app = FastAPI()
    app = FastAPI()
    app = FastAPI()
    app = FastAPI()
    app = FastAPI()
    app = FastAPI()
    app = FastAPI()
    app = FastAPI()
    app = FastAPI()
    app = FastAPI()
    app = FastAPI()
    app = FastAPI()