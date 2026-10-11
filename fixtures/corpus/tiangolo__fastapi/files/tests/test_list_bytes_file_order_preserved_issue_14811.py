from typing import Annotated
import anyio
import pytest
from fastapi import FastAPI, File
from fastapi.testclient import TestClient
from starlette.datastructures import UploadFile as StarletteUploadFile
    app = FastAPI()