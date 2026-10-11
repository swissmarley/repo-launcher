import json
from typing import Annotated
from fastapi import Cookie, FastAPI, Form, Header, Query
from fastapi.testclient import TestClient
from pydantic import Json
app = FastAPI()