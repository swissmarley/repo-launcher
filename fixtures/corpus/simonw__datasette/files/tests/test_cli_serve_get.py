import json
import textwrap
from click.testing import CliRunner
from datasette.cli import cli
from datasette.plugins import pm
        from datasette import hookimpl
        p for p in pm.get_plugins() if p.__name__ == "init_for_serve_with_get.py"
        from datasette import hookimpl
        p for p in pm.get_plugins() if p.__name__ == "bg_task_for_get.py"