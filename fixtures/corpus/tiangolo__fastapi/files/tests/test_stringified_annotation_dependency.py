from __future__ import annotations
from typing import TYPE_CHECKING, Annotated
import pytest
from fastapi import Depends, FastAPI
from fastapi.testclient import TestClient
from inline_snapshot import snapshot
    from collections.abc import AsyncGenerator
    app = FastAPI()