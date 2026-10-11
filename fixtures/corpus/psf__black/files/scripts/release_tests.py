import unittest
from pathlib import Path
from shutil import rmtree
from tempfile import TemporaryDirectory
from typing import Any
from unittest.mock import Mock, patch
from release import SourceFiles, is_valid_version  # type: ignore
if __name__ == "__main__":