import os
from flask import Flask
    app = Flask(__name__, instance_relative_config=True)
    from . import db
    from . import auth
    from . import blog