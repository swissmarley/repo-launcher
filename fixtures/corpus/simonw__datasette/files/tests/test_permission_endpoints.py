import pytest
import pytest_asyncio
from datasette.app import Datasette
    from datasette.database import Database
    import json
    from datasette import hookimpl
    from datasette.permissions import PermissionSQL
        __name__ = "TestPermissionPlugin"