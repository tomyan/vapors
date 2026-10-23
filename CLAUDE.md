# vapors

GPU-rendered UI framework in Rust. Design: `docs/` (arrives in S7); until
then the design lives in `~/projects/touche/UI-FRAMEWORK-BRIEF.md` and
the factory plan in `~/projects/touche/FACTORY.md`.

## Commands

```
cargo xtask --help        every factory command
cargo xtask setup         install pinned tools and the pre-push hook
cargo xtask ci            what CI runs: tools, fmt, clippy, wasm, unit, gpu; fastest first
cargo xtask lint          fmt and clippy only
cargo xtask wasm          clippy for the web package on wasm32-unknown-unknown
cargo xtask web           trunk release build of the web package into target/web
cargo xtask serve         put target/web on the tailnet over https; prints the URL
cargo xtask test          one tier (default unit)
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
cargo run -p vapors-elements-corpus --example layout_oracle [-- --check]
                          Chromium's boxes for the layout fixtures
bacon                     unit tier on every save
```

Tool versions: `.config/tools.toml`. Toolchain: `rust-toolchain.toml`.
Test deadlines: `.config/nextest.toml`.

## Rules

- Tests first. A check is proven by breaking what it guards, watching
  it fail, restoring. Say so in the commit.
- A tier that cannot run fails and names what is missing. Never skip.
- Files about 200 lines (checked: the `files` step of `cargo xtask lint`,
  `[files] max_lines` in `.config/factory.toml`, allow with a reason);
  functions 10 to 15 lines; no `unsafe`.
- Warnings are errors in CI (`-D warnings`, clippy pedantic).
- Every failure message names the fix and the reproducing command.

## Slice loop

1. Branch `area/sN-short-name` from `main`.
2. Red, green, refactor. Commit.
3. `cargo xtask ci` green locally.
4. Pull request. An independent review agent reads the diff with the
   "Failure must be loud" rules and no author context; findings are
   verified before action.
5. Auto-merge when CI is green. Squash. Branch deleted.
6. Update the plan; the PR description is the handoff.
