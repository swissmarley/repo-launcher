from typing import Annotated
import pytest
from fastapi import FastAPI
from fastapi.testclient import TestClient
from inline_snapshot import snapshot
from pydantic import BaseModel
    from fastapi import Body
    from pydantic import Discriminator, Tag
    app = FastAPI()