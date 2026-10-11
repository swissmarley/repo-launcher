from collections.abc import Callable
import pytest
from datasette.app import Datasette
from datasette.permissions import PermissionSQL
from datasette.utils.permissions import resolve_permissions_from_catalog
    import tempfile
    from datasette.database import Database