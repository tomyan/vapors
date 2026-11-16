#!/bin/sh
# The tables of property 7's record (docs/perf/first-frame-<date>.md),
# from the outputs scripts/first-frame-runs.sh keeps; the reading and
# every refusal are in scripts/first_frame_table.py.
#
# Usage: scripts/first-frame-table.sh <revision> <dir>/release-*.txt <dir>/judged-*.txt
# where <revision> is `git describe --always --dirty --abbrev=12` of the
# commit measured (the runs script passes it).
set -eu
exec python3 -I "$(dirname "$0")/first_frame_table.py" "$@"
