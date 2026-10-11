import logging
import secrets
import subprocess
from pathlib import Path
import yaml
from github import Github
from pydantic import BaseModel, SecretStr
from pydantic_settings import BaseSettings
if __name__ == "__main__":