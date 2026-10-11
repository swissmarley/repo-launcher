import pathlib
import pytest
from click.testing import CliRunner
from black import jupyter_dependencies_are_installed, main
from tests.util import get_case_path