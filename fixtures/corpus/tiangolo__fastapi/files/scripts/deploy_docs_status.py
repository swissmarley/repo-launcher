import logging
import re
from typing import Literal
from github import Auth, Github
from pydantic import BaseModel, SecretStr
from pydantic_settings import BaseSettings
if __name__ == "__main__":