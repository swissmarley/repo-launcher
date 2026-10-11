import runpy
import subprocess
import sys
from pathlib import Path
from unittest.mock import MagicMock
import pytest
        runpy.run_path(str(ACTION_MAIN), run_name="__main__")
        runpy.run_path(str(ACTION_MAIN), run_name="__main__")