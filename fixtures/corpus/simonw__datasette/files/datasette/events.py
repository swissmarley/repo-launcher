from abc import ABC, abstractproperty
from dataclasses import asdict, dataclass, field
from datetime import datetime, timezone
from datasette.hookspecs import hookimpl