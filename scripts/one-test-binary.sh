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
# - insta snapshots are renamed for their new module path; one whose
#   name does not begin with a test file's is left, and said so.
# - Cargo.toml gains `autotests = false` (after [package]'s last key)
#   and one [[test]] target, so a file under tests/ runs only when
#   main.rs declares it; main.rs's own test refuses a file it does not
#   declare, naming it and the fix.
# - A reproducing command `-p <package> --test <file>` anywhere in the
#   repository becomes `-p <package> --test tests <file>::`.
# - Every Rust file it writes is formatted, so a run on main gives the
#   tree a reviewer sees, and `cargo fmt --check` passes on it.
# Refused, naming the file and line: a manifest with its own [[test]]
# or [[bench]] targets, `harness` or `required-features`, and a nextest
# override naming one of the files' binaries.
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
written=$(mktemp)
trap 'rm -f "$written"' EXIT
python3 "$(dirname "$0")/one_test_binary.py" "$tests" "$manifest" .config/nextest.toml "$written"
# git as a hook would run it exports these; a git run here must not
# take the hook's repository (CLAUDE.md)
git_here() { env -u GIT_DIR -u GIT_WORK_TREE -u GIT_INDEX_FILE git "$@"; }
if [ -d "$tests/snapshots" ]; then
  for snap in "$tests"/snapshots/*.snap; do
    base=$(basename "$snap")
    module=${base%%__*}
    if [ -f "$tests/$module.rs" ]; then
      git_here mv "$snap" "$tests/snapshots/tests__$base"
    else
      echo "one-test-binary: left $snap: its name begins with no test file's ($module)"
    fi
  done
fi
# rustfmt orders main.rs's module lines its own way and folds the blank
# lines a removed `mod` leaves; formatted here, the tree is the script's
# output exactly and passes `cargo fmt --check`.
edition=$(sed -n 's/^edition = "\(.*\)"/\1/p' Cargo.toml | head -n 1)
rustfmt --edition "${edition:?the workspace Cargo.toml names no edition}" $(cat "$written") "$tests/main.rs"
echo "one-test-binary: $(ls "$tests"/*.rs | grep -vc '/main.rs$') files under $tests folded into tests/main.rs"
