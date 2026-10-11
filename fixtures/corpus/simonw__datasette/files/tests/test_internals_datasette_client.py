import httpx2
import pytest
import pytest_asyncio
from datasette.app import Datasette
    from datasette import Response, hookimpl
        __name__ = "test_in_client_plugin"
    from datasette import hookimpl
    from datasette.utils.asgi import Response
        __name__ = "test_in_client_skip_plugin"