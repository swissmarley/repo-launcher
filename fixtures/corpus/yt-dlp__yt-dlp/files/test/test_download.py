import os
import sys
import unittest
import collections
import hashlib
import json
from test.helper import (
import yt_dlp.YoutubeDL  # isort: split
from yt_dlp.extractor import get_info_extractor
from yt_dlp.networking.exceptions import HTTPError, TransportError
from yt_dlp.utils import (
        return f'{self._testMethodName} ({cls.__module__}.{cls.__name__}){f" [{add_ie}]" if add_ie else ""}:'
            if err.__class__.__name__ == expected_exception:
            return any(exc.__class__.__name__ == expected_exception for exc in err.exc_info)
        test_method.__name__ = tname
        setattr(TestDownload, test_method.__name__, test_method)
    test_method.__name__ = f'test_{name}_all'
    setattr(TestDownload, test_method.__name__, test_method)
if __name__ == '__main__':