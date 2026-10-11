from collections.abc import AsyncGenerator
from contextlib import asynccontextmanager
import pytest
from fastapi import APIRouter, FastAPI, Request
from fastapi.testclient import TestClient
from pydantic import BaseModel
    app = FastAPI()
    app = FastAPI(lifespan=lifespan)
    app = FastAPI(lifespan=lifespan)
    app = FastAPI(lifespan=lifespan)
    app = FastAPI(lifespan=lifespan)
    app = FastAPI(lifespan=lifespan)
    app = FastAPI()
    from collections.abc import Generator
    app = FastAPI(lifespan=lifespan)  # type: ignore[invalid-argument-type]  # ty: ignore[invalid-argument-type]
    app = FastAPI(lifespan=lifespan)  # type: ignore[invalid-argument-type]  # ty: ignore[invalid-argument-type]
    app = FastAPI(on_startup=[app_startup], on_shutdown=[app_shutdown])