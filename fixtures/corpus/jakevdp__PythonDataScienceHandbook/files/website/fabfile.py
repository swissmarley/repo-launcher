from fabric.api import *
import fabric.contrib.project as project
import os
import shutil
import sys
import SocketServer
from pelican.server import ComplexHTTPRequestHandler