#!/usr/bin/env bash
# The pinned Chromium's placeholder colours (tools.toml chromium.version),
# written to crates/elements/tests/placeholder_chromium/computed.txt,
# which tests/placeholder_colour.rs holds vapors to. Run by hand when the
# pin changes; routine CI reads the saved file and runs no browser.
set -euo pipefail
dir=crates/elements/tests/placeholder_chromium
version=$(sed -n 's/^chromium.version = "\(.*\)"/\1/p' .config/tools.toml)
chrome="$HOME/.local/share/factory/chromium-$version/chrome-linux/chrome"
[ -x "$chrome" ] || { echo "$chrome: missing; fix: cargo xtask setup" >&2; exit 1; }
profile=$(mktemp -d)
trap 'rm -rf "$profile"' EXIT
dom=$(timeout 60 "$chrome" --headless --no-sandbox --disable-gpu \
  --user-data-dir="$profile" --dump-dom "file://$PWD/$dir/probe.html")
rows=$(sed -n '/<pre id="out">/,/<\/pre>/p' <<<"$dom" | sed 's/<[^>]*>//g' | sed '/^$/d')
[ "$(wc -l <<<"$rows")" -eq 4 ] || { echo "expected 4 rows from $chrome, got: $rows" >&2; exit 1; }
{ echo "# $("$chrome" --version), $dir/probe.html"; echo "$rows"; } >"$dir/computed.txt"
cat "$dir/computed.txt"
