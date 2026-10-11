from io import BytesIO
from requests.adapters import BaseAdapter
from requests.models import Response
from requests.utils import get_encoding_from_headers
from httpie.plugins import TransportPlugin
from httpie.plugins.registry import plugin_manager
from .utils import HTTP_OK, http