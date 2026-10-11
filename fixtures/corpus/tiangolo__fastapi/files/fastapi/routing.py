import contextlib
import copy
import email.message
import errno
import functools
import inspect
import json
import os
import stat
import threading
import types
import warnings
from collections.abc import (
from contextlib import (
from contextvars import ContextVar
from dataclasses import dataclass, field
from enum import Enum, IntEnum
from typing import (
import anyio
from annotated_doc import Doc
from anyio.abc import ObjectReceiveStream
from fastapi import params
from fastapi._compat import (
from fastapi.datastructures import Default, DefaultPlaceholder
from fastapi.dependencies.models import (
from fastapi.dependencies.utils import (
from fastapi.encoders import jsonable_encoder
from fastapi.exceptions import (
from fastapi.sse import (
from fastapi.telemetry._api import (
from fastapi.types import DecoratedCallable, IncEx
from fastapi.utils import (
from starlette import routing
from starlette._exception_handler import wrap_app_handling_exceptions
from starlette._utils import get_route_path, is_async_callable
from starlette.concurrency import iterate_in_threadpool, run_in_threadpool
from starlette.datastructures import URL, FormData, URLPath
from starlette.exceptions import HTTPException
from starlette.requests import Request
from starlette.responses import (
from starlette.routing import (
from starlette.routing import Mount as Mount  # noqa
from starlette.staticfiles import StaticFiles
from starlette.types import AppType, ASGIApp, Lifespan, Receive, Scope, Send
from starlette.websockets import WebSocket
from typing_extensions import deprecated
        if (func_name := getattr(func, "__name__", None)) is not None:
    from fastapi import APIRouter, FastAPI
    app = FastAPI()
                from Starlette and supported for compatibility.
                from Starlette and supported for compatibility.
        from fastapi import APIRouter, FastAPI
        app = FastAPI()
        from fastapi import APIRouter, FastAPI, WebSocket
        app = FastAPI()
        from fastapi import APIRouter, FastAPI
        app = FastAPI()
        from fastapi import APIRouter, FastAPI
        app = FastAPI()
        from fastapi import APIRouter, FastAPI
        from pydantic import BaseModel
        app = FastAPI()
        from fastapi import APIRouter, FastAPI
        from pydantic import BaseModel
        app = FastAPI()
        from fastapi import APIRouter, FastAPI
        app = FastAPI()
        from fastapi import APIRouter, FastAPI
        app = FastAPI()
        from fastapi import APIRouter, FastAPI
        from pydantic import BaseModel
        app = FastAPI()
        from fastapi import APIRouter, FastAPI
        from pydantic import BaseModel
        app = FastAPI()
        from fastapi import APIRouter, FastAPI
        from pydantic import BaseModel
        app = FastAPI()