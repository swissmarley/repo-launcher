import argparse
import os
import platform
import sys
import socket
from typing import List, Optional, Union, Callable
import requests
from pygments import __version__ as pygments_version
from requests import __version__ as requests_version
from . import __version__ as httpie_version
from .cli.constants import OUT_REQ_BODY
from .cli.nested_json import NestedJSONSyntaxError
from .client import collect_messages
from .context import Environment, LogLevel
from .downloads import Downloader
from .models import (
from .output.models import ProcessingOptions
from .output.writer import write_message, write_stream, write_raw_data, MESSAGE_SEPARATOR_BYTES
from .plugins.registry import plugin_manager
from .status import ExitStatus, http_status_to_exit_status
from .utils import unwrap_context
from .internal.update_warnings import check_updates
from .internal.daemon_runner import is_daemon_mode, run_daemon_task
        env.log_error(f'{type(e).__name__}: {msg}')
    from .cli.definition import parser