import sys
import pytest
import responses
from httpie.encoding import UTF8
from httpie.output.formatters.xml import parse_xml, pretty_xml
from .fixtures import XML_FILES_PATH, XML_FILES_VALID, XML_FILES_INVALID, XML_DATA_RAW, XML_DATA_FORMATTED
from .utils import http, DUMMY_URL