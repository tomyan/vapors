# How Vapors changes are verified

An independent reviewer examines each proposed change. The local merge gate runs on the proposed revision in a clean worktree and posts its result on the pull request. Check that the revision in the passing gate result matches the revision under review.

The CI command checks formatting, linting, the WebAssembly build and tests including GPU rendering. A test tier reports a failure when its required environment is unavailable. GitHub Actions runs separately on pushes. The project uses the local gate for its merge decision, so a merge alone does not establish success on a hosted runner.

To inspect the exact checks, see the [CI workflow](../.github/workflows/ci.yml) or run `cargo xtask --help` from the checkout. Nightly quality signals are advisory.
