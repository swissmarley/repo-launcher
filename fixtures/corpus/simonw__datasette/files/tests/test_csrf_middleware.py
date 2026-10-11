import pluggy
import pytest
from datasette import hookimpl
from datasette.csrf import CrossOriginProtectionMiddleware, _install_legacy_csrftoken
        __name__ = "legacy-skip-csrf-plugin"
    throwaway.register(plugin, name=LegacyPlugin.__name__)