import json
import os
import secrets
from collections.abc import Iterable
from functools import lru_cache
from os import sep as pathsep
from pathlib import Path
from typing import Annotated
import git
import typer
import yaml
from github import Github
from pydantic_ai import Agent
from rich import print
from scripts.doc_parsing_utils import check_translation
from scripts.translation_git import (
if __name__ == "__main__":