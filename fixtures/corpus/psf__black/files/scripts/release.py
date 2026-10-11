from __future__ import annotations
import argparse
import logging
import re
import sys
from datetime import datetime
from pathlib import Path
from subprocess import run
from packaging.version import InvalidVersion, Version
LOG = logging.getLogger(__name__)
if __name__ == "__main__":  # pragma: no cover