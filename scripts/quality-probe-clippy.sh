#!/usr/bin/env bash
# Writes xtask/fixtures/quality_probe.clippy.tsv: clippy's own
# `too_many_lines` and `cognitive_complexity` counts for every function
# of xtask/fixtures/quality_probe.rs.txt, by building the fixture as a
# crate in a temporary directory and running clippy with both thresholds
# at 1. xtask's fixture test holds its counts to this file. Rerun on a
# toolchain bump, or when the fixture changes; commit the result.
set -euo pipefail
root=$(cd "$(dirname "$0")/.." && pwd)
fixture=$root/xtask/fixtures/quality_probe.rs.txt
out=$root/xtask/fixtures/quality_probe.clippy.tsv
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
mkdir -p "$tmp/crate/src" "$tmp/conf"
cp "$fixture" "$tmp/crate/src/lib.rs"
cp "$root/rust-toolchain.toml" "$tmp/crate/"
printf '[package]\nname = "quality_probe"\nversion = "0.0.0"\nedition = "2024"\n[lints.clippy]\ncognitive_complexity = "warn"\ntoo_many_lines = "warn"\n' > "$tmp/crate/Cargo.toml"
printf 'too-many-lines-threshold = 1\ncognitive-complexity-threshold = 1\n' > "$tmp/conf/clippy.toml"
# Lines reported as `src/lib.rs:LINE:COL: message (N/1)`. Every fixture
# function has its signature and body brace on its `fn` line, so both
# lints point at that line; a closure's own report sits on a later line
# and is not a function's.
(cd "$tmp/crate" && CLIPPY_CONF_DIR=$tmp/conf cargo clippy --all-targets --message-format=short 2>&1) \
  | grep -E "too many lines|cognitive complexity" \
  | sed -E 's/^src\/lib.rs:([0-9]+):[0-9]+: warning: this function has too many lines \(([0-9]+)\/1\).*/\1 lines \2/; s/^src\/lib.rs:([0-9]+):[0-9]+: warning: the function has a cognitive complexity of \(([0-9]+)\/1\).*/\1 complexity \2/' \
  > "$tmp/reports"
{
  echo "# name<TAB>lines<TAB>complexity: clippy's counts for quality_probe.rs.txt, written by scripts/quality-probe-clippy.sh"
  echo "# toolchain: $(rustc --version), $(cargo clippy --version)"
  grep -nE '^\s*(pub )?fn [a-z_]+' "$fixture" | sed -E 's/^([0-9]+):\s*(pub )?fn ([a-z_]+).*/\1 \3/' | while read -r line name; do
    lines=$(awk -v l="$line" '$1==l && $2=="lines" {print $3; exit}' "$tmp/reports")
    cc=$(awk -v l="$line" '$1==l && $2=="complexity" {print $3; exit}' "$tmp/reports")
    # unreported means not over 1: a one-line body, a straight-line function
    printf '%s\t%s\t%s\n' "$name" "${lines:-1}" "${cc:-1}"
  done
} > "$out"
echo "wrote $out ($(grep -vc '^#' "$out") functions)"
