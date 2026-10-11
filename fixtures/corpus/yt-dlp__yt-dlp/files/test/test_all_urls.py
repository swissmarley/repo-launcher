import os
import sys
import unittest
import collections
from test.helper import gettestcases
from yt_dlp.extractor import FacebookIE, YoutubeIE, gen_extractors
                if type(ie).__name__ in ('GenericIE', tc['name'] + 'IE'):
                    self.assertTrue(ie.suitable(url), f'{type(ie).__name__} should match URL {url!r}')
                        f'{type(ie).__name__} should not match URL {url!r} . That URL belongs to {tc["name"]}.')
            name_accu[ie.IE_NAME.lower()].append(type(ie).__name__)
if __name__ == '__main__':