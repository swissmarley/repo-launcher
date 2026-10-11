import asyncio
from typing import ClassVar
import pytest
from datasette.extras import Extra, ExtraRegistry, ExtraScope
    from datasette.views.table_extras import table_extra_registry
    assert cls.description, f"{cls.__name__} is missing a description"
    from datasette.views.table_extras import table_extra_registry