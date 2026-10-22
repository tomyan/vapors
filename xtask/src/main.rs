//! The factory: every build, test and check command for this repository.
//!
//! `cargo xtask --help` lists the commands. Each prints one line per
//! fact and, on failure, what was expected, what was found, how to fix
//! it and the exact command that reproduces it.

mod ci;
mod command;
mod config;
mod deadline;
mod dev;
mod files;
mod files_walk;
mod gpu_info;
mod pixels;
mod report;
mod screenshot;
mod serve;
mod setup;
mod steps;
mod stream;
mod tailscale;
mod tools;
mod trunk;
mod trunk_process;
mod web;
mod xvfb;

use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand, ValueEnum};

/// Factory commands for the vapors workspace.
#[derive(Parser)]
#[command(name = "cargo xtask", version, about)]
struct Cli {
    /// Print the full output of every command, not only the failure lines.
    #[arg(long, global = true)]
    verbose: bool,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
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
    /// Check every source file is within `[files] max_lines` (the `files` step).
    #[command(hide = true)]
    CheckFiles,
    /// Write docs/gpu-limits.md from the adapters and limits wgpu reports here.
    GpuInfo,
    /// Run a windowed example under Xvfb and save its frame as a PNG.
    Screenshot {
        /// Example (or, with --bin, binary) of the package named in .config/factory.toml.
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
    Web,
    /// Serve target/web on the tailnet over https and print the URL.
    Serve,
    /// Rebuild the web package on save (trunk serve, debug) behind the tailnet https proxy.
    Dev,
    /// Check formatting and lints without running tests.
    Lint,
    /// Run one test tier (default: unit).
    Test {
        /// Which tier to run.
        #[arg(long, value_enum, default_value_t = Tier::Unit)]
        tier: Tier,
    },
}

/// A group of tests with the same requirements.
#[derive(Clone, Copy, ValueEnum)]
enum Tier {
    /// Needs nothing beyond the toolchain. Runs on every save.
    Unit,
    /// Needs a GPU adapter; `VAPORS_GPU_ADAPTER` names which. Fails, never skips, without one.
    Gpu,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let reporter = report::Reporter::new(cli.verbose);
    let steps = match cli.command {
        Command::Setup { ci } => return setup::run(ci),
        Command::CheckTools => return setup::check(),
        Command::CheckFiles => return files::run(),
        Command::GpuInfo => return gpu_info::run(),
        Command::Screenshot {
            example,
            bin,
            out,
            expect,
            example_args,
        } => {
            let target = screenshot::Target {
                name: example,
                is_binary: bin,
            };
            return screenshot::run(&target, &out, expect.as_deref(), &example_args);
        }
        Command::Web => return web::run(),
        Command::Serve => return serve::run(),
        Command::Dev => return dev::run(&reporter),
        Command::Ci => match web::web_package() {
            Ok(package) => steps::ci(&package),
            Err(message) => return fail("ci", &message),
        },
        Command::Wasm => match web::web_package() {
            Ok(package) => steps::wasm_lint(&package),
            Err(message) => return fail("wasm", &message),
        },
        Command::Lint => steps::lint(),
        Command::Test { tier: Tier::Unit } => steps::unit_tests(),
        Command::Test { tier: Tier::Gpu } => steps::gpu_tests(),
    };
    ci::run(&steps, &reporter)
}

fn fail(command: &str, message: &str) -> ExitCode {
    println!("FAIL {command}: {message}");
    ExitCode::FAILURE
}
