import requests
from .fixtures import BIN_FILE_PATH, BIN_FILE_CONTENT, BIN_FILE_PATH_ARG
from httpie.output.streams import BINARY_SUPPRESSED_NOTICE
from .utils import MockEnvironment, http