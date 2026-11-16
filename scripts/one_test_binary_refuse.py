"""How scripts/one-test-binary.sh refuses."""

import sys


def refuse(message):
    sys.exit(f"one-test-binary: {message}")
