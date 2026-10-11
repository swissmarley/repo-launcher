import os
from collections.abc import Awaitable, Callable, Coroutine, Sequence
from enum import Enum
from typing import Annotated, Any, Literal, TypeVar
from annotated_doc import Doc
from fastapi import routing
from fastapi.datastructures import Default, DefaultPlaceholder
from fastapi.exception_handlers import (
from fastapi.exceptions import RequestValidationError, WebSocketRequestValidationError
from fastapi.logger import logger
from fastapi.middleware.asyncexitstack import AsyncExitStackMiddleware
from fastapi.openapi.docs import (
from fastapi.openapi.utils import get_openapi
from fastapi.params import Depends
from fastapi.telemetry import TelemetryConfig
from fastapi.telemetry._asgi import (
from fastapi.types import DecoratedCallable, IncEx
from fastapi.utils import generate_unique_id
from starlette.applications import Starlette
from starlette.datastructures import State
from starlette.exceptions import HTTPException
from starlette.middleware import Middleware
from starlette.middleware.base import BaseHTTPMiddleware
from starlette.middleware.errors import ServerErrorMiddleware
from starlette.middleware.exceptions import ExceptionMiddleware
from starlette.requests import Request
from starlette.responses import HTMLResponse, JSONResponse, Response
from starlette.routing import BaseRoute
from starlette.types import ASGIApp, ExceptionHandler, Lifespan, Receive, Scope, Send
from typing_extensions import deprecated
class FastAPI(Starlette):
    from fastapi import FastAPI
    app = FastAPI()
                app = FastAPI(telemetry={"tracing": False})
                from Starlette and supported for compatibility.
                from Starlette and supported for compatibility.
                from fastapi import FastAPI
                app = FastAPI(title="ChimichangApp")
                from fastapi import FastAPI
                app = FastAPI(summary="Deadpond's favorite app. Nuff said.")
                from fastapi import FastAPI
                app = FastAPI(
                from fastapi import FastAPI
                app = FastAPI(version="0.0.1")
                from fastapi import FastAPI
                app = FastAPI(openapi_url="/api/v1/openapi.json")
                from fastapi import FastAPI
                app = FastAPI(openapi_tags=tags_metadata)
                from fastapi import FastAPI
                app = FastAPI(
                from fastapi import Depends, FastAPI
                from .dependencies import func_dep_1, func_dep_2
                app = FastAPI(dependencies=[Depends(func_dep_1), Depends(func_dep_2)])
                from fastapi import FastAPI
                from fastapi.responses import ORJSONResponse
                app = FastAPI(default_response_class=ORJSONResponse)
                from fastapi import FastAPI
                app = FastAPI(redirect_slashes=True)  # the default
                from fastapi import FastAPI
                app = FastAPI(docs_url="/documentation", redoc_url=None)
                from fastapi import FastAPI
                app = FastAPI(docs_url="/documentation", redoc_url="redocumentation")
                app = FastAPI(terms_of_service="http://example.com/terms/")
                app = FastAPI(
                app = FastAPI(
                from fastapi import FastAPI
                app = FastAPI(root_path="/api/v1")
                from fastapi import FastAPI
                app = FastAPI(root_path_in_servers=False)
                from pydantic import BaseModel
                from fastapi import FastAPI
                app = FastAPI(openapi_external_docs=external_docs)
                from fastapi import FastAPI
                app = FastAPI()
            from fastapi.telemetry._runtime import lifespan
        from fastapi import FastAPI
        app = FastAPI()
        from fastapi import FastAPI, WebSocket
        app = FastAPI()
                from fastapi import Depends, FastAPI
                from .dependencies import get_token_header
                from .internal import admin
                app = FastAPI()
                from fastapi import FastAPI
                from .internal import old_api
                app = FastAPI()
                from fastapi import FastAPI
                from .internal import old_api
                app = FastAPI()
                from fastapi import FastAPI
                from fastapi.responses import ORJSONResponse
                from .internal import old_api
                app = FastAPI()
        from fastapi import FastAPI
        from .users import users_router
        app = FastAPI()
        from fastapi import FastAPI
        app = FastAPI()
        from fastapi import FastAPI
        from pydantic import BaseModel
        app = FastAPI()
        from fastapi import FastAPI
        from pydantic import BaseModel
        app = FastAPI()
        from fastapi import FastAPI
        app = FastAPI()
        from fastapi import FastAPI
        app = FastAPI()
        from fastapi import FastAPI, Response
        app = FastAPI()
        from fastapi import FastAPI
        from pydantic import BaseModel
        app = FastAPI()
        from fastapi import FastAPI
        app = FastAPI()
        import time
        from typing import Awaitable, Callable
        from fastapi import FastAPI, Request, Response
        app = FastAPI()
        from fastapi import FastAPI, Request
        from fastapi.responses import JSONResponse
        app = FastAPI()