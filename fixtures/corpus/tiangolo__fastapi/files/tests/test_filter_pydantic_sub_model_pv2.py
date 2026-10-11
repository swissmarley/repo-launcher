import pytest
from dirty_equals import HasRepr
from fastapi import Depends, FastAPI
from fastapi.exceptions import ResponseValidationError
from fastapi.testclient import TestClient
from inline_snapshot import snapshot
    from pydantic import BaseModel, ValidationInfo, field_validator
    app = FastAPI()
                        "operationId": "get_model_a_model__name__get",