from __future__ import annotations
import asyncio
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path
from typing import Any, Optional
import black.concurrency as concurrency
from black import Mode, WriteBack
from black.report import Report