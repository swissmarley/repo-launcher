import hashlib
from datasette import hookimpl
from datasette.utils import to_css_class
from datasette.utils.asgi import BadRequest, Response