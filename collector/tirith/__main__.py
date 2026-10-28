"""Allow ``python -m tirith`` as an alias for the ``tirith`` console script."""

import sys

from tirith.cli import main

if __name__ == "__main__":
    sys.exit(main())
