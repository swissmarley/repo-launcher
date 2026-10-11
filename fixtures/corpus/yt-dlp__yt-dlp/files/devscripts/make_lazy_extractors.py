import os
import sys
from inspect import getsource
from devscripts.utils import get_filename_args, read_file, write_file
from yt_dlp.extractor import import_extractors
from yt_dlp.extractor.common import InfoExtractor, SearchInfoExtractor
from yt_dlp.globals import extractors
        yield build_lazy_ie(ie, ie.__name__, attr_base)
            names.append(ie.__name__)
    assert ies[-1].__name__ == 'GenericIE', 'Last IE must be GenericIE'
            for b in sorted(bases, key=lambda x: x.__name__):
                    assert b.__name__ != 'GenericIE', 'Cannot inherit from GenericIE'
    }.get(base.__name__, base.__name__) for base in ie.__bases__)
if __name__ == '__main__':