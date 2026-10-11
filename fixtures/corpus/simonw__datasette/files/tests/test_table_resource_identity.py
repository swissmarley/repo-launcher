import uuid
from unittest.mock import AsyncMock
import pytest
from datasette import hookimpl
from datasette.app import Datasette
from datasette.default_permissions import restrictions_allow_action
from datasette.permissions import Action, PermissionSQL, _permission_check_cache
from datasette.resources import QueryResource, TableResource
from datasette.utils.actions_sql import explain_permission_for_resource
from datasette.utils.asgi import Forbidden
from datasette.utils.permissions import gather_permission_sql_from_hooks