import logging
import random
import sys
import time
from pathlib import Path
from typing import Any, cast
import httpx
from github import Github
from pydantic import BaseModel, SecretStr
from pydantic_settings import BaseSettings
if __name__ == "__main__":