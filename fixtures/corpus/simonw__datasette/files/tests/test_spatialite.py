import pytest
from datasette.app import Datasette
from datasette.utils import SPATIALITE_FUNCTIONS, SpatialiteNotFound, find_spatialite
from .utils import has_load_extension