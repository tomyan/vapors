"""The tables of property 7's record (scripts/first-frame-table.sh).

Reads the verbatim outputs of `vapors-perf --first-frame <rows>` that
scripts/first-frame-runs.sh keeps, named `release-<rows>-<n>-<cold|warm>.txt`
(times) and `judged-<rows>-<n>.txt` (instructions). Every line of a file
must have a known shape; anything else is refused naming the file and
line, as is a file whose content disagrees with its name, a run of
another revision than the one given, a software adapter, a load of 1.5
or more at a run's start or end, or a size without three runs of each.
"""

import os
import re
import statistics
import sys

MAX_LOAD = 1.5  # crates/perf/src/scene_judge.rs, the finish line's method
RUNS = 3
NAME = re.compile(r"(release)-(\d+)-(\d+)-(cold|warm)\.txt|(judged)-(\d+)-(\d+)\.txt")
PROFILES = {
    "release": "release (the shipped profile)",
    "judged": "judged (release, codegen-units 1, no override the build could see)",
}
SHAPES = [
    ("adapter", r"Adapter: .+"),
    ("adapter type", r"Adapter type: (\w+)"),
    ("timestamps", r"Timestamp queries: (true|false)"),
    ("revision", r"Revision: (\S+)"),
    ("profile", r"Profile: (.+)"),
    ("cache", r"Shader cache: .+, (?:empty|\d+ entries) at the start \((cold|warm)\)"),
    ("load start", r"Load average at the start: ([\d.]+)"),
    ("load end", r"Load average at the end: ([\d.]+)"),
    ("summary", r"First frame, (\d+) rows: mount [\d.]+ µs, App::start to idle [\d.]+ µs, "
                r"frame 0 to idle [\d.]+ µs \(to submit [\d.]+ µs\); mount to frame 0 idle ([\d.]+) µs\."),
    ("header", r"\| part \| phase \| µs \| instructions \|"),
    ("separator", r"\|---\|---\|---\|---\|"),
    ("row", r"\| (mount|start|frame) \| ([a-z ]+) \| ([\d.]+) \| (\d+|none) \|"),
    ("blank", r""),
]
ONCE = ["adapter", "adapter type", "timestamps", "revision", "profile", "cache",
        "load start", "load end", "summary", "header", "separator"]


def refuse(where, why, fix):
    sys.exit(f"{where}: {why}; fix: {fix}")


def shape_of(line):
    for name, pattern in SHAPES:
        found = re.fullmatch(pattern, line)
        if found:
            return name, found.groups()
    return None, None


def read(path):
    """The file's fields by shape, each once, and its rows, each once."""
    fields, rows = {}, {}
    for at, line in enumerate(open(path, encoding="utf-8").read().splitlines(), 1):
        where = f"{path}:{at}: {line!r}"
        name, groups = shape_of(line)
        if name is None:
            refuse(where, "a line of no shape a --first-frame output has", RERUN_ONE)
        if name == "row":
            key = (groups[0], groups[1])
            if key in rows:
                refuse(where, f"a second {key[0]} {key[1]} row", RERUN_ONE)
            rows[key] = (float(groups[2]), None if groups[3] == "none" else int(groups[3]))
        elif name != "blank":
            if name in fields:
                refuse(where, f"a second {name} line", RERUN_ONE)
            fields[name] = (at, groups)
    missing = [name for name in ONCE if name not in fields]
    if missing:
        refuse(path, f"no {', '.join(missing)} line", RERUN_ONE)
    return fields, rows


def judged(path, name, fields, revision):
    """Checks a file's content against its name and the method."""
    kind, rows = name[0], int(name[1])
    line = lambda field: f"{path}:{fields[field][0]}"
    checks = [
        ("adapter type", fields["adapter type"][1][0] in ("IntegratedGpu", "DiscreteGpu"),
         "the adapter is not a GPU, so its frame times the CPU", "run on the dev machine's GPU, VAPORS_GPU_ADAPTER unset"),
        ("revision", fields["revision"][1][0] == revision,
         f"the run is of {fields['revision'][1][0]}, not {revision}", "rerun at that revision, on a clean tree"),
        ("profile", fields["profile"][1][0] == PROFILES[kind],
         f"the file is named {kind} but its profile is {fields['profile'][1][0]}", RERUN_ALL),
        ("summary", int(fields["summary"][1][0]) == rows,
         f"the file is named for {rows} rows", RERUN_ALL),
    ]
    if kind == "release":
        checks.append(("cache", fields["cache"][1][0] == name[3],
                       f"the file is named {name[3]} but the cache was {fields['cache'][1][0]}", RERUN_ALL))
    for when in ("load start", "load end"):
        checks.append((when, float(fields[when][1][0]) < MAX_LOAD,
                       f"the 1-minute load is {fields[when][1][0]}, not under {MAX_LOAD}", QUIET))
    for field, ok, why, fix in checks:
        if not ok:
            refuse(line(field), why, fix)


RERUN_ONE = "rerun scripts/first-frame-runs.sh; a --first-frame output is read as printed"
RERUN_ALL = "make the files with scripts/first-frame-runs.sh, which names them"
QUIET = "rerun under `cargo xtask quiet 30 -- scripts/first-frame-runs.sh <dir>`"


def runs(paths, revision):
    """Each file's (kind, rows, cache) and its fields and rows."""
    out = []
    for path in paths:
        found = NAME.fullmatch(os.path.basename(path))
        if not found:
            refuse(path, "not a name scripts/first-frame-runs.sh gives", RERUN_ALL)
        name = [g for g in found.groups() if g is not None]
        fields, rows = read(path)
        judged(path, name, fields, revision)
        cache = name[3] if name[0] == "release" else "warm"
        out.append(((name[0], int(name[1]), cache), fields, (path, rows)))
    return out


def grouped(all_runs):
    groups = {}
    for key, fields, rows in all_runs:
        groups.setdefault(key, []).append((fields, rows))
    sizes = sorted({key[1] for key in groups})
    for rows in sizes:
        for key in [("release", rows, "cold"), ("release", rows, "warm"), ("judged", rows, "warm")]:
            count = len(groups.get(key, []))
            if count != RUNS:
                sys.exit(f"{key[0]} {key[2]} at {rows} rows has {count} runs, not {RUNS}; fix: {RERUN_ALL}")
    return groups, sizes


def median(values):
    return statistics.median(values)


def cell(each, part, phase, index):
    """The median of one table cell over `each` run; a run without the
    row, or a judged run without its count, is refused."""
    values = []
    for _, (path, rows) in each:
        if (part, phase) not in rows:
            refuse(path, f"no {part} {phase} row", RERUN_ONE)
        value = rows[(part, phase)][index]
        if value is None:
            refuse(path, f"the {part} {phase} row has no instruction count", RERUN_ONE)
        values.append(value)
    return median(values)


TIME_COLUMNS = [("mount", "all"), ("start", "all"), ("start", "style"), ("start", "layout"),
                ("start", "paint"), ("frame", "all"), ("frame", "to idle")]
COUNT_COLUMNS = [("mount", "all"), ("start", "style"), ("start", "layout"), ("start", "paint"),
                 ("start", "write"), ("frame", "all")]


def main(revision, paths):
    if revision.endswith("-dirty"):
        sys.exit(f"revision {revision} is of a dirty tree, which no commit holds; fix: commit, then rerun scripts/first-frame-runs.sh")
    groups, sizes = grouped(runs(paths, revision))
    print(f"Revision {revision}. Each figure is the median of {RUNS} runs, one page a process; "
          "mount to frame 0 idle is each run's own sum (mount + App::start to idle + frame 0 to idle), "
          "then its median. Times on release; instructions on judged.\n")
    heads = " | ".join(f"{part} {phase} µs" for part, phase in TIME_COLUMNS)
    print(f"| rows | cache | mount to frame 0 idle µs | {heads} |")
    print("|---|---|---|" + "---|" * len(TIME_COLUMNS))
    for rows in sizes:
        for cache in ("cold", "warm"):
            each = groups[("release", rows, cache)]
            total = median(float(fields["summary"][1][1]) for fields, _ in each)
            cells = " | ".join(f"{cell(each, p, ph, 0):.1f}" for p, ph in TIME_COLUMNS)
            print(f"| {rows} | {cache} | {total:.1f} | {cells} |")
    heads = " | ".join(f"{part} {phase}" for part, phase in COUNT_COLUMNS)
    print(f"\n| rows | {heads} |")
    print("|---|" + "---|" * len(COUNT_COLUMNS))
    for rows in sizes:
        each = groups[("judged", rows, "warm")]
        print(f"| {rows} | " + " | ".join(str(int(cell(each, p, ph, 1))) for p, ph in COUNT_COLUMNS) + " |")


if __name__ == "__main__":
    if len(sys.argv) < 3:
        sys.exit("usage: scripts/first-frame-table.sh <revision> <run output> ...")
    main(sys.argv[1], sys.argv[2:])
