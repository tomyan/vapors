#!/usr/bin/env bash
# What the lanes build's wasm holds, by function name (twiggy): the
# layout-named code of vapors-elements and Taffy against the whole.
# Names come from an unstripped build in target/named; sizes are
# shallow function bytes, so inlined code counts where it landed.
# Reproduce, from the repository root:
#   crates/perf/layout-spike/attribute.sh
set -euo pipefail
cd "$(dirname "$0")"
CARGO_INCREMENTAL=0 CARGO_PROFILE_RELEASE_STRIP=false CARGO_TARGET_DIR=target/named \
  cargo build --offline --quiet --release --target wasm32-unknown-unknown --bin size --no-default-features --features lanes
wasm=target/named/wasm32-unknown-unknown/release/size.wasm
twiggy top -n 100000 -f csv "$wasm" | python3 -c '
import csv, sys
rows = [r for r in csv.reader(sys.stdin)][1:]
code = [(n, int(s)) for n, s, *_ in rows if "segment" not in n and "subsection" not in n and "section" not in n]
def total(*keys):
    return sum(s for n, s in code if all(k in n for k in keys))
print("function bytes, all:", sum(s for _, s in code))
print("taffy:", total("taffy"))
print("vapors_elements, all:", total("vapors_elements"))
print("vapors_elements, layout in the name:", total("vapors_elements", "layout"))
print("vapors_elements, lane in the name:", total("vapors_elements", "lane"))
'
