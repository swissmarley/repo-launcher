import pytest
from fastapi import Body, Cookie, FastAPI, Header, Path, Query
from fastapi.exceptions import FastAPIDeprecationWarning
from fastapi.testclient import TestClient
from inline_snapshot import snapshot
from pydantic import BaseModel, ConfigDict
    app = FastAPI()