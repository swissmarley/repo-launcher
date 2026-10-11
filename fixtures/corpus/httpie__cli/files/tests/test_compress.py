import base64
import zlib
from .fixtures import FILE_PATH, FILE_CONTENT
from httpie.status import ExitStatus
from .utils import StdinBytesIO, http, HTTP_OK, MockEnvironment