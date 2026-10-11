import html
import json
import pathlib
from dataclasses import dataclass, field
import pytest
from datasette.app import TEMPLATE_BASE_CONTEXT, Datasette
from datasette.extras import ExtraScope
from datasette.fixtures import write_fixture_database
from datasette.template_contexts import PAGES, documented_context_keys
from datasette.views import Context
@pytest.mark.parametrize("klass", PAGES.values(), ids=lambda klass: klass.__name__)
        ), f"{klass.__name__}.{context_field.name} is missing documentation"
@pytest.mark.parametrize("klass", PAGES.values(), ids=lambda klass: klass.__name__)
    assert klass.__doc__, f"{klass.__name__} is missing a docstring"
    ), f"{klass.__name__} is missing a documented_template"
    from datasette.views import from_extra
    from datasette.views.table_extras import CountExtra
    from datasette.views import from_extra
    from datasette.views import from_extra
    from datasette.plugins import DEFAULT_PLUGINS, pm
        title = "{} page".format(klass.__name__.removesuffix("Context"))