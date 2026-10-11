import pytest
import shutil
import json
from httpie.sessions import SESSIONS_DIR_NAME
from httpie.status import ExitStatus
from httpie.cli.options import PARSER_SPEC_VERSION
from tests.utils import DUMMY_HOST, httpie
from tests.fixtures import SESSION_FILES_PATH, SESSION_FILES_NEW, SESSION_FILES_OLD, read_session_file