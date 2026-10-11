import io
from pathlib import Path
from typing import cast
import pytest
from fastapi import FastAPI, UploadFile
from fastapi.datastructures import Default, DefaultPlaceholder
from fastapi.testclient import TestClient
    app = FastAPI()