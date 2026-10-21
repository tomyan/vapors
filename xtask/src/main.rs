//! The factory: every build, test and check command for this repository.
//!
//! `cargo xtask --help` lists the commands. Each prints one line per
//! fact and, on failure, what was expected, what was found, how to fix
//! it and the exact command that reproduces it.

mod ci;
mod command;
mod report;
mod steps;

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
    /// Run everything CI runs, in CI's order, stopping at the first failure.
    Ci,
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
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let reporter = report::Reporter::new(cli.verbose);
    let steps = match cli.command {
        Command::Ci => steps::ci(),
        Command::Lint => steps::lint(),
        Command::Test { tier: Tier::Unit } => steps::unit_tests(),
    };
    ci::run(&steps, &reporter)
}
