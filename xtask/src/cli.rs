//! The command line: `cargo xtask <command>`.

use std::path::PathBuf;

use clap::{Parser, Subcommand, ValueEnum};

/// Factory commands for the vapors workspace.
#[derive(Parser)]
#[command(name = "cargo xtask", version, about)]
pub struct Cli {
    /// Print the full output of every command, not only the failure lines.
    #[arg(long, global = true)]
    pub verbose: bool,
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// Install the pinned tools and the pre-push hook; say what was done.
    Setup {
        /// On a CI runner: install tools only, no git hook.
        #[arg(long)]
        ci: bool,
    },
    /// Run everything CI runs, in CI's order, stopping at the first failure.
    Ci,
    /// Check every pinned tool is installed at its pinned version.
    #[command(hide = true)]
    CheckTools,
    /// Check a crate in `[imports.<crate>]` is named only where listed (the `imports` step).
    CheckImports,
    /// Check no crate depends on what `[layers.forbid]` forbids it (the `layers` step).
    CheckLayers,
    /// Check every source file is within `[files] max_lines` (the `files` step).
    #[command(hide = true)]
    CheckFiles,
    /// Write docs/gpu-limits.md from the adapters and limits wgpu reports here.
    GpuInfo,
    /// Run a windowed example under Xvfb and save its frame as a PNG.
    Screenshot {
        /// Example (or, with --bin, binary) of a package in `[screenshot] packages`.
        example: String,
        /// Run a binary of that package instead of an example.
        #[arg(long)]
        bin: bool,
        /// Where to write the PNG.
        out: PathBuf,
        /// Fail unless the centre pixel is this colour, as `r,g,b`.
        #[arg(long)]
        expect: Option<String>,
        /// Arguments passed to the example after `--`, such as `--open file.png`.
        #[arg(last = true)]
        example_args: Vec<String>,
    },
    /// Lint the web package for wasm32-unknown-unknown (the `wasm` CI step).
    Wasm,
    /// Build the web package with trunk into target/web.
    Web {
        /// A debug build into target/web-debug, for `check_dev_keys`.
        #[arg(long)]
        debug: bool,
    },
    /// Build main's page and serve it on the tailnet where [serve] says; print the URL.
    Serve {
        /// Replace another target mounted where [serve] says.
        #[arg(long)]
        replace: bool,
    },
    /// trunk's post-build hook: version.json and `?v=<build>` on the
    /// page's own files in trunk's staging directory.
    StampPage,
    /// Send the walk's frames and facts to a model for the UX checklist's
    /// judged items (settings: `[ux]` in .config/factory.toml).
    UxReview {
        /// Build the request and print its estimated tokens and worst-case
        /// cost; send nothing.
        #[arg(long)]
        dry_run: bool,
        /// Also post the report as a comment on this pull request.
        #[arg(long)]
        pr: Option<u32>,
        /// Count `ux.model`'s input tokens for each frame size, low and
        /// high detail, and print its row for `ux_models`.
        #[arg(long, conflicts_with_all = ["dry_run", "pr"])]
        measure_image_cost: bool,
    },
    /// Rebuild the web package on save (trunk serve, debug) behind the tailnet https proxy.
    Dev {
        /// A key for the debug page, as `<provider>=<file>` (anthropic or
        /// openai); the file must be mode 600. Repeat for each provider.
        #[arg(long = "key", value_name = "PROVIDER=FILE")]
        keys: Vec<String>,
        /// Replace another target mounted on [dev] https.
        #[arg(long)]
        replace: bool,
    },
    /// Run every step of .github/workflows/ci.yml here, in order, ending `gate: pass <sha>`.
    Gate {
        /// Post the gate's last lines to this pull request (`--pr` alone:
        /// the PR whose head is this branch), checking its head is HEAD.
        #[arg(long, num_args = 0..=1, value_name = "N")]
        // Allowed: clap's form for a flag whose value may be left out
        // (`--pr` alone, `--pr N`), so the two cases need no sentinel.
        #[allow(clippy::option_option)]
        pr: Option<Option<u64>>,
    },
    /// Check formatting and lints without running tests.
    Lint,
    /// Check dependencies with cargo-deny: advisories, bans, licences, sources.
    Deny,
    /// Fail naming each file where ./xtask differs from <sibling>/xtask.
    CheckXtaskTwin {
        /// The sibling checkout, such as ../vapors.
        sibling: PathBuf,
    },
    /// Run one test tier (default: unit).
    Test {
        /// Which tier to run.
        #[arg(long, value_enum, default_value_t = Tier::Unit)]
        tier: Tier,
    },
}

/// A group of tests with the same requirements.
#[derive(Clone, Copy, ValueEnum)]
pub enum Tier {
    /// Needs nothing beyond the toolchain. Runs on every save.
    Unit,
    /// Needs a GPU adapter; `VAPORS_GPU_ADAPTER` names which. Fails, never skips, without one.
    Gpu,
}
