from collections.abc import Callable, Sequence
from typing import Annotated, Any, Literal
from annotated_doc import Doc
from fastapi import params
from fastapi._compat import Undefined
from fastapi.datastructures import _Unset
from fastapi.openapi.models import Example
from pydantic import AliasChoices, AliasPath
from typing_extensions import deprecated
    from typing import Annotated
    from fastapi import FastAPI, Path
    app = FastAPI()
    from typing import Annotated
    from fastapi import Depends, FastAPI
    app = FastAPI()
    from typing import Annotated
    from fastapi import Security, FastAPI
    from .db import User
    from .security import get_current_active_user
    app = FastAPI()