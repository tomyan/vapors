#!/bin/sh
# The tables of docs/perf's scene-run records, made from the runs'
# verbatim outputs (scripts/scene-runs.sh): each run's revision, cache,
# over-budget count, frame 0 and statistics, then the median of the
# runs' statistics with the worst frame of any run beside it. A file
# without one of those lines, or runs of different revisions, are
# refused naming the file.
#
# Usage: scripts/scene-run-table.sh <run output> ...
set -eu
[ $# -gt 0 ] || { echo "usage: scripts/scene-run-table.sh <run output> ..." >&2; exit 1; }
python3 -I - "$@" <<'PY'
import re, statistics, sys

TIMES = ["frame (CPU)", "GPU time", "frame to idle"]

def field(path, text, pattern):
    found = re.search(pattern, text, re.M)
    if not found:
        sys.exit(f"{path}: no line matching {pattern!r}; is it a --scene-run output?")
    return found.groups()

def run(path):
    text = open(path, encoding="utf-8").read()
    row = {t: [float(v) for v in field(path, text, rf"^\| {re.escape(t)} \| ([\d.]+) \| ([\d.]+) \| ([\d.]+) \|$")] for t in TIMES}
    return {
        "name": path.rsplit("/", 1)[-1].removesuffix(".txt"),
        "revision": field(path, text, r"^Revision: (\S+)$")[0],
        "cache": field(path, text, r"^Shader cache: .*\((cold|warm)\)$")[0],
        "over": int(field(path, text, r"over 16\.7 ms to idle: (\d+)\.")[0]),
        "frame0": field(path, text, r"^\| 0 \| ([\d.]+) \| ([\d.]+) \| ([\d.]+) \|$"),
        "times": row,
    }

runs = [run(path) for path in sys.argv[1:]]
revisions = {r["revision"] for r in runs}
if len(revisions) != 1:
    sys.exit(f"the runs are of different revisions {sorted(revisions)}; table runs of one")
print(f"Revision {revisions.pop()}.\n")
print("| run | cache | over budget | frame 0 CPU / GPU / to idle µs | " + " | ".join(f"{t} median / p99 / max µs" for t in TIMES) + " |")
print("|---|---|---|---|" + "---|" * len(TIMES))
for r in runs:
    stats = " | ".join(" / ".join(f"{v:.1f}" for v in r["times"][t]) for t in TIMES)
    print(f"| {r['name']} | {r['cache']} | {r['over']} | {' / '.join(r['frame0'])} | {stats} |")
print("\n| time, over these runs | median of medians µs | median of p99s µs | median of maxima µs | worst frame µs |")
print("|---|---|---|---|---|")
for t in TIMES:
    columns = list(zip(*(r["times"][t] for r in runs)))
    medians = " | ".join(f"{statistics.median_low(c):.1f}" for c in columns)
    print(f"| {t} | {medians} | {max(columns[2]):.1f} |")
PY
