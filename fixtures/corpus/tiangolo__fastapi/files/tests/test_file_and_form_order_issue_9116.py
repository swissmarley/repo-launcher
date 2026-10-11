from pathlib import Path
from typing import Annotated
import pytest
from fastapi import FastAPI, File, Form
from fastapi.testclient import TestClient
app = FastAPI()