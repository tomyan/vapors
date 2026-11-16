#!/usr/bin/env bash
# Folds a crate's integration tests into one test binary
# (docs/plans/crate-structure.md, section 5, step 1): each file under
# tests/ becomes a module of tests/main.rs, one link of the crate's
# closure in place of one a file.
#
# - A shared directory module (tests/<name>/mod.rs) that files declare
#   plainly (`mod <name>;`) is declared once in main.rs; each file's
#   line becomes `use crate::<name>;`, or goes where the file never
#   names it.
# - A file under tests/ that several files include by #[path] is
#   declared once in main.rs too, as a module named for its path
#   (support/surface.rs is `support_surface`), so it compiles once; each
#   including file's two lines become `use crate::<module> as <its
#   name>;`. In the included file, a `use crate::<name>` or `use
#   super::<name>` naming what its includers call <name> becomes that
#   module's crate path; includers that disagree, or a `super::` naming
#   anything else of theirs, are refused.
# - A file one test file includes stays its module, its `use crate::`
#   lines now `use super::`. A #[path] outside tests/ stays as it is.
# - insta snapshots are renamed for their new module path.
# - Cargo.toml gains `autotests = false` and one [[test]] target, so a
#   file under tests/ runs only when main.rs declares it; main.rs's own
#   test refuses a file it does not declare, naming it and the fix.
#
# Usage: scripts/one-test-binary.sh <crate directory>, e.g.
#   scripts/one-test-binary.sh crates/elements
set -euo pipefail
crate="${1:?usage: scripts/one-test-binary.sh <crate directory>}"
tests="$crate/tests"
manifest="$crate/Cargo.toml"
[ -d "$tests" ] || { echo "one-test-binary: $tests is not a directory"; exit 1; }
[ ! -e "$tests/main.rs" ] || { echo "one-test-binary: $tests/main.rs exists; fix: the crate is folded already"; exit 1; }
! grep -q '^autotests' "$manifest" || { echo "one-test-binary: $manifest sets autotests already; fix: fold by hand or not at all"; exit 1; }
command -v rustfmt >/dev/null || { echo "one-test-binary: rustfmt is not on PATH; fix: add ~/.cargo/bin to PATH"; exit 1; }
python3 "$(dirname "$0")/one_test_binary.py" "$tests" "$manifest"
if [ -d "$tests/snapshots" ]; then
  for snap in "$tests"/snapshots/*.snap; do
    base=$(basename "$snap")
    module=${base%%__*}
    [ -f "$tests/$module.rs" ] && git mv "$snap" "$tests/snapshots/tests__$base"
  done
fi
# rustfmt orders the module lines its own way; written in its order, the
# file passes `cargo fmt --check` as the script leaves it.
edition=$(sed -n 's/^edition = "\(.*\)"/\1/p' Cargo.toml | head -n 1)
rustfmt --edition "${edition:?the workspace Cargo.toml names no edition}" "$tests/main.rs"
echo "one-test-binary: $(ls "$tests"/*.rs | grep -vc '/main.rs$') files under $tests folded into tests/main.rs"
