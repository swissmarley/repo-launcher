import json
import pytest
from fastapi import BackgroundTasks, Depends, FastAPI
from fastapi.responses import StreamingResponse
from fastapi.testclient import TestClient
app = FastAPI()