#!/usr/bin/env bash
# Wasm size of the size program with each option alone and with none:
# raw bytes and gzip -9 bytes (no brotli or wasm-opt in this sandbox).
# Reproduce, from the repository root:
#   crates/perf/layout-spike/size.sh
set -euo pipefail
cd "$(dirname "$0")"
export CARGO_INCREMENTAL=0
printf '| features | wasm bytes | gzip -9 bytes |\n|---|---|---|\n'
for features in "" taffy lanes fixed generated "taffy,fixed,generated"; do
  cargo build --offline --quiet --release --target wasm32-unknown-unknown --bin size --no-default-features \
    ${features:+--features "$features"} 1>&2
  wasm=target/wasm32-unknown-unknown/release/size.wasm
  printf '| %s | %s | %s |\n' "${features:-none}" "$(stat -c %s "$wasm")" "$(gzip -9 -c "$wasm" | wc -c)"
done
