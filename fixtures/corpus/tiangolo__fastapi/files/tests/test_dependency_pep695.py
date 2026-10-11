from typing import Annotated
from fastapi import Depends, FastAPI
from fastapi.testclient import TestClient
from typing_extensions import TypeAliasType
    app = FastAPI()