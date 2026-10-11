import os
import sys
import platform
from PyInstaller.__main__ import run as run_pyinstaller
from devscripts.utils import read_version
        'yt_dlp/__main__.py',
    from PyInstaller.utils.win32.versioninfo import (
        from PyInstaller.utils.win32.versioninfo import SetVersion
        from PyInstaller.utils.win32.versioninfo import write_version_info_to_executable as SetVersion
if __name__ == '__main__':