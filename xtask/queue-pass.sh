#!/bin/bash
# One merge-queue pass for <repo> (touche or vapors), run by the
# queue@<repo> systemd user timer (xtask/systemd/). It runs from a
# worktree kept at origin/main, ~/.cache/w-queue/<repo>-main/<repo>, so
# the queue's own code is always main's, never a PR's; the queue then
# rebases and gates in its run worktree ([queue] run). Pinned siblings
# (touche's ../vapors) are moved to their pins first. Extra arguments
# (--dry-run) go to `cargo xtask queue`.
set -euo pipefail
repo="${1:?usage: queue-pass.sh <repo> [--dry-run]}"
shift
main="$HOME/.cache/w-queue/$repo-main/$repo"
git() { env -u GIT_DIR -u GIT_WORK_TREE -u GIT_INDEX_FILE git "$@"; }
if [ ! -d "$main/.git" ] && [ ! -f "$main/.git" ]; then
  echo "FAIL queue-pass: $main is not a worktree; fix: git -C ~/projects/$repo worktree add --detach $main origin/main" >&2
  exit 1
fi
cd "$main"
git fetch -q origin main
git checkout -q --detach origin/main
if [ -f .config/vapors-rev ]; then
  pin=$(grep -v '^#' .config/vapors-rev | head -1)
  git -C ../vapors fetch -q origin "$pin"
  git -C ../vapors checkout -q --detach "$pin"
fi
exec cargo xtask queue "$@"
