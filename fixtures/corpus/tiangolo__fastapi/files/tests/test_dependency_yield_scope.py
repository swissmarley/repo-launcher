import json
from typing import Annotated, Any
import pytest
from fastapi import APIRouter, Depends, FastAPI, HTTPException
from fastapi.exceptions import FastAPIError
from fastapi.responses import StreamingResponse
from fastapi.testclient import TestClient
app = FastAPI()
    app = FastAPI(dependencies=[Depends(raise_after_yield, scope="function")])
    app = FastAPI(dependencies=[Depends(raise_after_yield, scope="request")])