from __future__ import annotations
import os
import sys
import enum
import itertools
import json
import logging
import re
from collections import defaultdict
from dataclasses import dataclass
from functools import lru_cache
from pathlib import Path
from devscripts.utils import read_file, run_process, write_file
logger = logging.getLogger(__name__)
    import argparse
if __name__ == '__main__':