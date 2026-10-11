from collections.abc import Mapping, Sequence
from typing import Annotated, Any, TypedDict
from annotated_doc import Doc
from pydantic import BaseModel, create_model
from starlette.exceptions import HTTPException as StarletteHTTPException
from starlette.exceptions import WebSocketException as StarletteWebSocketException
    from fastapi import FastAPI, HTTPException
    app = FastAPI()
    from typing import Annotated
    from fastapi import (
    app = FastAPI()