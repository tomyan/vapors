//! The factory: every build, test and check command for this repository.
//!
//! `cargo xtask --help` lists the commands. Each prints one line per
//! fact and, on failure, what was expected, what was found, how to fix
//! it and the exact command that reproduces it.

mod ci;
mod cli;
mod command;
mod config;
mod deadline;
mod dev;
mod dev_keys;
mod dev_keys_server;
mod files;
mod files_walk;
mod gate;
mod gate_action;
mod gate_expression;
mod gate_plan;
mod gate_report;
mod gate_run;
mod gate_step;
mod gate_steps;
mod gate_twin;
mod gate_workflow;
mod gate_yaml;
mod gpu_info;
#[cfg(test)]
#[path = "hook_tests.rs"]
mod hook_tests;
mod layers;
mod layers_graph;
mod pixels;
mod report;
#[cfg(test)]
#[path = "report_timeout_tests.rs"]
mod report_timeout_tests;
mod sccache;
mod screenshot;
mod screenshot_package;
mod serve;
mod setup;
mod setup_install;
mod stamp_page;
mod step;
mod steps;
mod stop;
mod stream;
mod tailscale;
mod tools;
mod trunk;
mod trunk_process;
mod twin;
mod web;
mod web_version;
mod xvfb;
mod xvfb_display;

use std::process::ExitCode;

use clap::Parser;

use cli::{Cli, Command, Tier};

fn main() -> ExitCode {
    let cli = Cli::parse();
    let reporter = report::Reporter::new(cli.verbose);
    let steps = match cli.command {
        Command::Setup { ci } => return setup::run(ci),
        Command::CheckTools => return setup::check(),
        Command::CheckFiles => return files::run(),
        Command::CheckLayers => return layers::run(),
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
        Command::Web { debug } => return web::run(debug),
        Command::Serve => return serve::run(),
        Command::StampPage => return stamp_page::run(),
        Command::CheckXtaskTwin { sibling } => return twin::run(&sibling),
        Command::Dev { keys } => return dev::run(&reporter, &keys),
        Command::Ci => match web::web_package() {
            Ok(package) => return ci_with_hit_rate(&steps::ci(&package), &reporter),
            Err(message) => return fail("ci", &message),
        },
        Command::Wasm => match web::web_package() {
            Ok(package) => steps::wasm_lint(&package),
            Err(message) => return fail("wasm", &message),
        },
        Command::Gate => return gate::run(&reporter),
        Command::Lint => steps::lint(),
        Command::Deny => steps::deny_check(),
        Command::Test { tier: Tier::Unit } => steps::unit_tests(),
        Command::Test { tier: Tier::Gpu } => steps::gpu_tests(),
    };
    ci::run(&steps, &reporter)
}

/// `cargo xtask ci`, then sccache's hit rate over the run; sccache must
/// answer before and after.
fn ci_with_hit_rate(steps: &[steps::Step], reporter: &report::Reporter) -> ExitCode {
    let before = match sccache::stats() {
        Ok(stats) => stats,
        Err(why) => return fail("sccache", &why),
    };
    let code = ci::run(steps, reporter);
    match sccache::stats() {
        Ok(after) => match sccache::hit_rate(before, after) {
            Ok(line) => println!("{line}"),
            Err(why) => return fail("sccache", &why),
        },
        Err(why) => return fail("sccache", &why),
    }
    code
}

fn fail(command: &str, message: &str) -> ExitCode {
    println!("FAIL {command}: {message}");
    ExitCode::FAILURE
}
