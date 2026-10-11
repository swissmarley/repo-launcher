import warnings
import pytest
from fastapi import FastAPI
from fastapi.exceptions import FastAPIDeprecationWarning
from fastapi.responses import ORJSONResponse  # ty: ignore[deprecated]
from fastapi.testclient import TestClient
from sqlalchemy.sql.elements import quoted_name
    app = FastAPI(default_response_class=ORJSONResponse)  # ty: ignore[deprecated]