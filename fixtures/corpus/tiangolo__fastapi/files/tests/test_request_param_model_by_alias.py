from dirty_equals import IsPartialDict
from fastapi import Cookie, FastAPI, Header, Query
from fastapi.testclient import TestClient
from pydantic import BaseModel, Field
app = FastAPI()