import inspect
import sys
from collections.abc import AsyncGenerator, Generator
from functools import wraps
import pytest
from fastapi import Depends, FastAPI
from fastapi.concurrency import iterate_in_threadpool, run_in_threadpool
from fastapi.testclient import TestClient
    from inspect import iscoroutinefunction
    from asyncio import iscoroutinefunction
app = FastAPI()