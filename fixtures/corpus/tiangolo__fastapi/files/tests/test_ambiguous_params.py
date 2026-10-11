from typing import Annotated
import pytest
from fastapi import Depends, FastAPI, Path
from fastapi.param_functions import Query
from fastapi.testclient import TestClient
app = FastAPI()