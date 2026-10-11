import sys

import six


def main() -> int:
    print(f"hello from py-cli on Python {sys.version_info.major}.{sys.version_info.minor} (six {six.__version__})")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
