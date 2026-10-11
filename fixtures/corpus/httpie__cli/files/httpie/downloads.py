import mimetypes
import os
import re
from mailbox import Message
from time import monotonic
from typing import IO, Optional, Tuple
from urllib.parse import urlsplit
import requests
from .models import HTTPResponse, OutputOptions
from .output.streams import RawStream
from .context import Environment
        from httpie.output.ui.rich_progress import (