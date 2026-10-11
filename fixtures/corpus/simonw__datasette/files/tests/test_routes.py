import pytest
import pytest_asyncio
from datasette.app import Database, Datasette
from datasette.utils import resolve_routes
            view.__name__ == expected_name or view.view_class.__name__ == expected_name