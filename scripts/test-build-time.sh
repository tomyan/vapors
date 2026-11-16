#!/usr/bin/env bash
# How long a package's test targets take to build after one change to
# its lib (docs/plans/crate-structure.md, section 5, step 1): touches
# the lib's root, times `cargo test -p <package> --no-run --timings`,
# and prints the wall time, the load average before and after, and the
# units cargo's timings report: the package's own and the sum of all.
# Run it once beforehand so dependencies are built.
#
# Usage: scripts/test-build-time.sh <package> <crate directory>
set -euo pipefail
package="${1:?usage: scripts/test-build-time.sh <package> <crate directory>}"
crate="${2:?usage: scripts/test-build-time.sh <package> <crate directory>}"
export CARGO_INCREMENTAL=0
echo "load before: $(cut -d' ' -f1-3 /proc/loadavg)"
touch "$crate/src/lib.rs"
start=$(date +%s.%N)
cargo test -p "$package" --no-run --timings >/dev/null 2>&1 || { echo "test-build-time: cargo test -p $package --no-run failed; reproduce: cargo test -p $package --no-run"; exit 1; }
end=$(date +%s.%N)
echo "load after: $(cut -d' ' -f1-3 /proc/loadavg)"
echo "wall: $(echo "$end - $start" | bc) s"
python3 - "$package" target/cargo-timings/cargo-timing.html <<'PY'
import json, re, sys
package, report = sys.argv[1], sys.argv[2]
units = json.loads(re.search(r"const UNIT_DATA = (\[.*?\]);", open(report).read(), re.S).group(1))
own = [u for u in units if u["name"] == package]
tests = [u for u in own if "test" in u.get("target", "")]
print(f"units: {len(units)}, {sum(u['duration'] for u in units):.1f} s summed")
print(f"{package}: {len(own)} units, {sum(u['duration'] for u in own):.1f} s; test units {len(tests)}, {sum(u['duration'] for u in tests):.1f} s")
PY
