#!/bin/sh
# The tables of property 7's record (docs/perf/first-frame-<date>.md),
# made from the verbatim outputs of `vapors-perf 9 --mount`
# (scripts/first-frame-runs.sh): per size, the median over the runs of
# each run's median mount, first frame and its phases in µs, then the
# median instructions per phase. Refused, naming the file and line: a
# run of another revision, one built without the shipped profile, on a
# software or unclassified adapter, with a 1-minute load of 1.5 or more
# at its start or end, or without one of the lines read here.
#
# Usage: scripts/first-frame-table.sh <run output> ... (three runs, the
# finish-line method)
set -eu
[ $# -eq 3 ] || { echo "usage: scripts/first-frame-table.sh <run1> <run2> <run3>: the method is three runs" >&2; exit 1; }
python3 -I - "$@" <<'PY'
import re, statistics, sys

SIZES = [1000, 3000, 10000]
PHASES = ["update", "style", "layout", "paint", "accessibility", "draw"]
MAX_LOAD = 1.5  # crates/perf/src/scene_judge.rs, the finish line's method

QUIET = "rerun under `cargo xtask quiet 20 -- scripts/first-frame-runs.sh <dir>`"
SHAPE = "make the runs with scripts/first-frame-runs.sh at a revision whose --mount prints it"

def refuse(path, lines, at, why, fix=SHAPE):
    line = f":{at + 1}: {lines[at]!r}" if at is not None else ""
    sys.exit(f"{path}{line}: {why}; fix: {fix}")

def find(path, lines, pattern):
    for at, line in enumerate(lines):
        found = re.fullmatch(pattern, line)
        if found:
            return at, found.groups()
    refuse(path, lines, None, f"no line matching {pattern!r}; is it a `vapors-perf --mount` output?")

def judged(path, lines):
    at, (kind,) = find(path, lines, r"Adapter type: (\w+)")
    if kind not in ("IntegratedGpu", "DiscreteGpu"):
        refuse(path, lines, at, f"the adapter is of type {kind}, not a GPU, so its draw times the CPU",
               "run on the dev machine's GPU, with VAPORS_GPU_ADAPTER unset")
    at, _ = find(path, lines, r"Profile: (.*)")
    if not lines[at].startswith("Profile: release (the shipped profile)"):
        refuse(path, lines, at, "not built with release, the profile property 7 is timed on",
               "scripts/first-frame-runs.sh builds it (cargo build --release -p vapors-perf)")
    for when in ("start", "end"):
        at, (load,) = find(path, lines, rf"Load average at the {when}: ([\d.]+) [\d.]+ [\d.]+")
        if float(load) >= MAX_LOAD:
            refuse(path, lines, at, f"the 1-minute load at the {when} is {load}, not under {MAX_LOAD}", QUIET)
    at, (revision,) = find(path, lines, r"Revision: (\S+)")
    if revision.endswith("-dirty"):
        refuse(path, lines, at, "built from a dirty tree, so the figures trace to no commit",
               "commit, then run scripts/first-frame-runs.sh, which refuses a dirty tree")
    return revision

def times(path, lines, rows):
    at, _ = find(path, lines, rf"### {rows} rows, first frame")
    phases = {}
    for line in lines[at + 1:]:
        if line.startswith("### "):
            break
        cell = re.fullmatch(r"\| ([A-Za-z ()]+) \| ([\d.]+) \| [\d.]+ \| [\d.]+ \|", line)
        if cell:
            phases[cell[1]] = float(cell[2])
    for phase in ["frame (CPU)"] + PHASES:
        if phase not in phases:
            refuse(path, lines, at, f"the {rows}-row table has no {phase} row")
    phases["mount"] = float(find(path, lines, rf"Mount, {rows} rows: median mount ([\d.]+) µs over \d+ kept mounts\.")[1][0])
    return phases

def instructions(path, lines):
    at, _ = find(path, lines, r"### Mount and first frame, median instructions")
    table = {}
    for line in lines[at + 1:]:
        cells = re.fullmatch(r"\| ([A-Za-z ()]+) \| (\d+) \| (\d+) \| (\d+) \|.*", line)
        if cells:
            table[cells[1]] = [int(c) for c in cells.groups()[1:]]
        elif table and not line.startswith("|"):
            break
    if "mount" not in table:
        refuse(path, lines, at, "the instructions table has no mount row")
    return table

def run(path):
    lines = open(path, encoding="utf-8").read().splitlines()
    revision = judged(path, lines)
    return revision, {rows: times(path, lines, rows) for rows in SIZES}, instructions(path, lines)

runs = [run(path) for path in sys.argv[1:]]
revisions = sorted({r[0] for r in runs})
if len(revisions) != 1:
    sys.exit(f"the runs are of different revisions {revisions}; table three runs of one")
print(f"Revision {revisions[0]}, {len(runs)} runs; each figure is the median of the runs' medians.\n")
columns = ["mount", "frame (CPU)"] + PHASES
print("| rows | mount and first frame µs | " + " | ".join(f"{c} µs" for c in columns) + " |")
print("|---|---|" + "---|" * len(columns))
for rows in SIZES:
    each = [r[1][rows] for r in runs]
    total = statistics.median(t["mount"] + t["frame (CPU)"] for t in each)
    cells = " | ".join(f"{statistics.median(t[c] for t in each):.1f}" for c in columns)
    print(f"| {rows} | {total:.1f} | {cells} |")
print("\n| phase | " + " | ".join(f"{rows} rows, instructions" for rows in SIZES) + " |")
print("|---|" + "---|" * len(SIZES))
for phase in runs[0][2]:
    counts = [r[2].get(phase) for r in runs]
    if None in counts:
        sys.exit(f"the {phase} row is missing from a run's instructions table; table runs of one revision")
    print(f"| {phase} | " + " | ".join(str(int(statistics.median(c))) for c in zip(*counts)) + " |")
PY
