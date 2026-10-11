from typing import Any, cast
from fastapi import FastAPI, UploadFile
from fastapi._compat import (
from fastapi._compat.shared import is_bytes_sequence_annotation
from fastapi.testclient import TestClient
from pydantic import BaseModel, ConfigDict
from pydantic.fields import FieldInfo
    from fastapi._compat import v2
    app = FastAPI()
    app = FastAPI()
    from fastapi._compat import v2
    from fastapi._compat import v2
    from typing import Union
    from fastapi._compat import v2