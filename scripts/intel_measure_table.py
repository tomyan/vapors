"""Mesa's INTEL_MEASURE=draw CSV of a scene run as a Markdown table
(queue item 13b, docs/perf/render-profile-13b.md): the GPU time of each
pipeline (its vertex and fragment shader hashes) in each kind of render
pass, as a mean per frame over the frames the capture holds.

A pass is `scene` when it draws indexed (the scene renderer's meshes and
ground), `page` otherwise; events outside a render pass (copies,
clears, blits) are `outside passes`. A frame is one `page` pass: the
page pass is the last of each frame. The first `skip` frames are left
out (first-use costs).

Usage: python3 -I scripts/intel_measure_table.py <csv> [skip frames]
Refuses a file without INTEL_MEASURE's columns, or with no page pass,
naming the file and line.
"""
import collections
import csv
import sys

COLUMNS = {"batch", "renderpass", "type", "count", "vs", "fs", "time_us"}


def rows(path):
    with open(path, encoding="utf-8") as file:
        reader = csv.DictReader(file)
        missing = COLUMNS - set(reader.fieldnames or [])
        if missing:
            sys.exit(f"{path}:1: no column {sorted(missing)}; is it INTEL_MEASURE=draw output?")
        for line, row in enumerate(reader, start=2):
            try:
                float(row["time_us"])
            except ValueError:
                sys.exit(f"{path}:{line}: time_us {row['time_us']!r} is not a number")
            yield row


def passes(path):
    """Each render pass's events in order, keyed (batch, renderpass);
    events outside a pass (renderpass 0) each on their own."""
    grouped = collections.OrderedDict()
    for row in rows(path):
        key = (row["batch"], row["renderpass"]) if row["renderpass"] != "0" else (row["batch"], "out", len(grouped))
        grouped.setdefault(key, []).append(row)
    return list(grouped.values())


def kind(events):
    if events[0]["renderpass"] == "0":
        return "outside passes"
    return "scene" if any(e["type"] == "draw indexed" for e in events) else "page"


def frames(all_passes, skip):
    """The passes split into frames, each ending at a page pass."""
    split, current = [], []
    for events in all_passes:
        current.append(events)
        if kind(events) == "page":
            split.append(current)
            current = []
    return split[skip:]


def table(path, skip):
    kept = frames(passes(path), skip)
    if not kept:
        sys.exit(f"{path}:1: no page pass after the first {skip} frames; nothing to report")
    times, draws = collections.Counter(), collections.Counter()
    for frame in kept:
        for events in frame:
            for e in events:
                name = e["type"] if e["type"] not in ("draw", "draw indexed") else f"{e['type']} vs {e['vs'][-8:]} fs {e['fs'][-8:]}"
                times[(kind(events), name)] += float(e["time_us"])
                draws[(kind(events), name)] += 1
    total = sum(times.values())
    n = len(kept)
    out = [f"{path}: {n} frames (first {skip} left out); mean GPU time per frame {total / n:.1f} µs\n",
           "| pass | event | per frame | mean µs per frame | share |", "|---|---|---|---|---|"]
    for key, time in times.most_common():
        out.append(f"| {key[0]} | {key[1]} | {draws[key] / n:.1f} | {time / n:.1f} | {100 * time / total:.0f}% |")
    return "\n".join(out)


if __name__ == "__main__":
    if len(sys.argv) not in (2, 3):
        sys.exit("usage: python3 -I scripts/intel_measure_table.py <csv> [skip frames]")
    print(table(sys.argv[1], int(sys.argv[2]) if len(sys.argv) == 3 else 20))
