from collections.abc import Callable
from typing import Annotated, Any
from annotated_doc import Doc
from fastapi.telemetry._api import _operation
from starlette.background import BackgroundTasks as StarletteBackgroundTasks
from typing_extensions import ParamSpec
    from fastapi import BackgroundTasks, FastAPI
    app = FastAPI()