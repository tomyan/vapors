# vapors

GPU-rendered UI framework in Rust. Design: `docs/` (arrives in S7); until
then the design lives in `~/projects/touche/UI-FRAMEWORK-BRIEF.md` and
the factory plan in `~/projects/touche/FACTORY.md`.

## Commands

```
cargo xtask --help        every factory command
cargo xtask setup         install pinned tools ([downloads] fetched and verified by hash) and the pre-push hook
cargo xtask ci            what CI runs: tools, fmt, clippy, wasm, unit, gpu; fastest first
cargo xtask deny          cargo-deny by deny.toml: advisories, bans, licences, sources
cargo xtask lint          fmt and clippy only
cargo xtask wasm          clippy for the web package on wasm32-unknown-unknown
cargo xtask web           trunk release build of the web package into target/web, with
                          brotli (.br) and gzip (.gz) copies of its wasm, js, css, html and json
cargo xtask serve         the release build of main on the tailnet where [serve] says (from a
                          clean checkout at origin/main after `cargo xtask web`; [serve] check
                          first, then a copy under ~/srv). Restarts the user unit
                          xtask-serve-<dir>.service: the hash-verified static-web-server
                          ([downloads] in tools.toml) on 127.0.0.1:[serve] port, sandboxed
                          (the copy, its config and the binary bound read-only, the rest of
                          the home directory empty), compressed copies, hashed files immutable,
                          the rest no-cache. It must own the port; /, every wasm and a script
                          must come compressed with their types over the loopback before
                          tailscale serve mounts it, and again through the tailnet URL after.
                          Needs lingering (loginctl enable-linger); prints the URL
cargo xtask test          one tier (default unit)
cargo xtask scaling       vapors-perf's scaling judge (`--scaling all`, every change) in
                          release on a clean tree; ends `scaling: pass|fail <sha>`
scripts/nightly-scaling.sh [--install]
                          that judge on origin/main in ~/.cache/w-nightly/vapors, a GitHub
                          issue per failing change, closed when it passes; --install writes
                          the systemd --user timer (03:00) and does not enable it:
                          systemctl --user enable --now vapors-nightly-scaling.timer
scripts/nightly-scaling-check.sh
                          its guards with a fake git and gh: network deadlines, the lock,
                          the runner's error quoted in its issue
scripts/nightly-scaling-issues-check.sh
                          its issues and run markers against scripts/fixtures/
                          nightly-scaling.log, with a fake gh
cargo xtask quiet 10 -- <command>
                          keep xtask's ci, test and gate runs from starting (each waits,
                          saying who holds it and when it expires) once running ones
                          finish, run the command, release; `quiet hold [minutes]` (15
                          by default, 60 at most) keeps it after returning, `quiet
                          release` ends it. The gate holds it for its run
cargo run -p vapors-app --example demo -- hello
                          an F2c demo in a window (crates/demos)
cargo xtask screenshot demo target/hello.png -- hello
                          the same under Xvfb, held to demos/hello.pixels
cargo run -p vapors-app --example idle_fps -- hello|counter|image
                          frames drawn in 3 s idle, natively (0 when it redraws on change)
scripts/web-idle-frames.sh target/web <new dir>
                          the same on the built page in headless Chromium
xvfb-run -a cargo run -p vapors-platform --example wake_window
                          Platform::waker in a window: 0 frames idle, 1 per wake
scripts/web-wake-frames.sh target/web <new dir>
                          the same on the built page (?demo=wake)
xvfb-run -a cargo run -p vapors-app --example surface_gradient -- --check
                          a gpu-surface in a window: 0 frames and renders idle, 2 frames
                          and 1 render per invalidate, LoopClosed after the loop ends
cargo run -p vapors-elements-corpus --example layout_oracle [-- --check]
                          Chromium's boxes for the layout fixtures
bacon                     unit tier on every save
```

Tool versions: `.config/tools.toml`. Toolchain: `rust-toolchain.toml`.
Test deadlines: `.config/nextest.toml`.

## Builds

Every build xtask starts compiles through sccache: one cache for every
worktree at `~/.cache/sccache` (50 GB). In CI each runner starts with
an empty cache (a GitHub Actions cache gave no gain over rust-cache's
target, #35). xtask refuses to build without it (`cargo xtask setup` installs it).
Incremental compilation is off for every build (`.cargo/config.toml`;
`CARGO_INCREMENTAL` set in the environment overrides it): xtask's builds
go through sccache, which needs it off, and an incremental target of a
plain build reached 104 GB. Plain `cargo` builds 4 jobs at once (`[build] jobs` in
`.cargo/config.toml`), as xtask's steps do; at most three `cargo xtask
ci` runs go at once on the machine, and the next waits for a slot,
naming who holds them. A worktree's `target` lives under
`~/.cache`, never the `/tmp` tmpfs. When its PR merges, empty it and keep
the directory:
`find <target dir> -mindepth 1 -maxdepth 1 -exec rm -rf {} +`.

## Rules

- Main changes only by PR merge. The pre-push hook refuses a push to
  `main`, before its checks, naming the fix. Branch protection refuses it
  too, but touche has none, and the hook is shared. `ALLOW_PUSH_TO_MAIN=1
  git push …` lets one through, and the hook prints that it did. Use it
  only when Tom or the lead asks for a direct push to main.
- Tests first. A check is proven by breaking what it guards, watching
  it fail, restoring. Say so in the commit.
- A tier that cannot run fails and names what is missing. Never skip.
- Files about 200 lines, functions 10 to 15 lines, no `unsafe`. The
  checks sit at the tail (`cargo xtask lint`: `[files] max_lines` in
  `.config/factory.toml`, clippy's `too_many_lines` and
  `cognitive_complexity` at clippy.toml's thresholds; over one, fix it
  or allow it at the site with a comment naming the one concept it
  holds). The band below is judged in review (`docs/review.md`);
  `cargo xtask quality` lists its worst, nightly and on demand, and
  never fails on a finding. A split made only to pass a number is a
  review finding.
- Warnings are errors in CI (`-D warnings`, clippy pedantic).
- Every failure message names the fix and the reproducing command.
- Where a service asks for a contact address (a crates.io User-Agent,
  for example), give the repository URL, https://github.com/tomyan/vapors.
  Never give a person's email address.

## Slice loop

1. Branch `area/sN-short-name` from `main`.
2. Red, green, refactor. Commit.
3. `cargo xtask ci` green locally.
4. Pull request. An independent review agent reads the diff with the
   "Failure must be loud" rules and `docs/review.md` (single
   responsibility, reaching through other objects, splits made to pass
   a number) and no author context; findings are verified before action.
5. Merge on the local gate, not GitHub CI (Tom, 2026-11-11, as touche
   since 2026-10-23): once the review's findings are in, update the
   branch onto current `main`, run `cargo xtask gate --pr <n>` at the
   PR's head in a clean worktree, which posts its last lines to the PR.
   It must end `gate: pass <head sha>`. Then merge: squash, branch
   deleted. GitHub CI still runs on every push but is not a required
   check; a red GitHub run on a merged head is investigated as a
   difference between the runner and this machine.
6. Update the plan; the PR description is the handoff.
