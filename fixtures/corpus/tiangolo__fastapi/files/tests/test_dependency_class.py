from collections.abc import AsyncGenerator, Generator
import pytest
from fastapi import Depends, FastAPI
from fastapi.testclient import TestClient
app = FastAPI()