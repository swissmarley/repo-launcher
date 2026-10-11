from collections.abc import AsyncGenerator, Generator
from functools import partial
from typing import Annotated
import pytest
from fastapi import Depends, FastAPI
from fastapi.testclient import TestClient
app = FastAPI()