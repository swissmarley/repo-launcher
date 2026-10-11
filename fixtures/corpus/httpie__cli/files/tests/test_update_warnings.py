import json
import tempfile
import time
from contextlib import suppress
from datetime import datetime
from pathlib import Path
import pytest
from httpie.internal.daemon_runner import STATUS_FILE
from httpie.internal.daemons import spawn_daemon
from httpie.status import ExitStatus
from .utils import PersistentMockEnvironment, http, httpie