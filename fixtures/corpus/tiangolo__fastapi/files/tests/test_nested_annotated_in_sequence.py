from typing import Annotated
from dirty_equals import IsList
from fastapi import FastAPI, Query
from fastapi.testclient import TestClient
from inline_snapshot import snapshot
from pydantic import Field
app = FastAPI()