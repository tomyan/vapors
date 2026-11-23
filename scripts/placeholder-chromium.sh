#!/usr/bin/env bash
# The pinned Chromium's placeholder colours (tools.toml chromium.version),
# written to crates/elements/tests/placeholder_chromium/computed.txt,
# which tests/placeholder_colour.rs holds vapors to. Run by hand when the
# pin changes; routine CI reads the saved file and runs no browser.
set -euo pipefail
dir=crates/elements/tests/placeholder_chromium
pin=$(sed -n 's/^chromium.version = "\(.*\)"/\1/p' .config/tools.toml)
chrome="$HOME/.local/share/factory/chromium-$pin/chrome-linux/chrome"
[ -x "$chrome" ] || { echo "$chrome: missing; fix: cargo xtask setup" >&2; exit 1; }
version=$("$chrome" --version)
profile=$(mktemp -d)
trap 'rm -rf "$profile"' EXIT
# 60 s: the whole script took 0.44 s here (2026-11-23); the rest is room
# for a cold start on a loaded machine.
dom=$(timeout 60 "$chrome" --headless --disable-gpu \
  --user-data-dir="$profile" --dump-dom "file://$PWD/$dir/probe.html")
rows=$(sed -n '/<pre id="out">/,/<\/pre>/p' <<<"$dom" | sed 's/<[^>]*>//g' | sed '/^$/d')
want=$(grep -c '<input ' "$dir/probe.html")
[ "$(wc -l <<<"$rows")" -eq "$want" ] || { echo "expected $want rows from $chrome, got: $rows" >&2; exit 1; }
printf '# chromium.version %s (%s), %s/probe.html\n%s\n' "$pin" "$version" "$dir" "$rows" >"$dir/computed.txt"
cat "$dir/computed.txt"
