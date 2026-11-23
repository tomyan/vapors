//! The factory: every build, test and check command for this repository.
//!
//! `cargo xtask --help` lists the commands. Each prints one line per fact and, on failure,
//! what was expected, what was found, how to fix it and the exact command that reproduces it.

mod api_ask;
mod api_budget;
mod api_build_drain;
mod api_build_report;
mod api_build_run;
mod api_build_setup;
mod api_clone;
mod api_curls;
mod api_fds;
mod api_fences;
mod api_fences_run;
mod api_fences_works;
mod api_host;
mod api_http;
mod api_inner;
mod api_json_strict;
mod api_limits;
mod api_mcp_inner;
mod api_paths;
mod api_prices;
mod api_push;
mod api_push_checks;
mod api_push_ci;
mod api_push_fetch;
mod api_push_history;
mod api_push_remote;
mod api_push_text;
mod api_refusals;
mod api_request_content;
mod api_request_policy;
mod api_sandbox;
mod api_server;
mod api_server_ask;
mod api_server_proxy;
mod api_session;
mod api_timed;
mod api_tool_prompt;
mod api_upstream;
mod api_usage;
mod builds;
mod builds_config;
mod builds_tree;
mod ci;
mod ci_shared;
mod cli;
mod command;
mod config;
mod config_line;
mod config_schema;
mod deadline;
mod dev;
mod dev_keys;
mod dev_keys_server;
mod download_install;
mod downloads;
mod files;
mod files_walk;
mod gate;
mod gate_action;
mod gate_deadline;
mod gate_expression;
mod gate_hold;
mod gate_hold_table;
mod gate_lock;
mod gate_lock_shared;
mod gate_lock_slots;
mod gate_plan;
mod gate_pr;
mod gate_pr_gh;
mod gate_pr_real;
mod gate_pr_text;
mod gate_report;
mod gate_run;
mod gate_state;
mod gate_step;
mod gate_steps;
mod gate_twin;
mod gate_workflow;
mod gate_worktree_lock;
mod gate_yaml;
mod gpu_info;
#[cfg(test)]
#[path = "hook_tests.rs"]
mod hook_tests;
mod imports;
mod imports_rules;
mod layers;
mod layers_graph;
mod load;
mod load_wait;
#[cfg(test)]
mod lock_child;
#[cfg(test)]
mod lock_child_handle;
mod lock_owner;
mod lock_place;
mod pixels;
mod precompress;
mod priority;
mod quality;
mod quality_attrs;
mod quality_chains;
mod quality_clippy;
mod quality_collect;
mod quality_complexity;
mod quality_distributions;
mod quality_join;
mod quality_lines;
mod quality_macros;
mod quality_measure;
mod quality_stats;
mod quality_thresholds;
mod queue;
mod queue_act;
mod queue_checks;
mod queue_clear;
mod queue_comments;
mod queue_config;
mod queue_decide;
#[cfg(test)]
mod queue_fake;
mod queue_gate_child;
mod queue_git;
#[cfg(test)]
mod queue_git_fixture;
mod queue_judge;
mod queue_list;
mod queue_lock;
mod queue_log;
mod queue_ops;
mod queue_pass;
mod queue_pins;
mod queue_process_tree;
mod queue_real;
mod queue_regate;
mod queue_remote;
mod queue_serve;
mod queue_state;
mod quiet_command;
mod quiet_lock;
mod quiet_wait;
mod report;
#[cfg(test)]
#[path = "report_timeout_tests.rs"]
mod report_timeout_tests;
mod run_tmp;
mod scaling;
mod sccache;
#[cfg(test)]
mod scratch;
mod screenshot;
mod screenshot_package;
mod serve;
mod serve_config;
mod serve_listen;
mod serve_mount;
mod serve_unit;
mod serve_verify_rules;
mod setup;
mod setup_install;
mod share;
mod stamp_page;
mod step;
mod steps;
mod steps_checks;
mod stop;
mod stream;
mod tailscale;
mod target_cap;
mod test_shared;
mod tools;
mod trunk;
mod trunk_process;
mod twin;
mod ux_answer;
mod ux_base64;
mod ux_call;
mod ux_checklist;
mod ux_config;
mod ux_count;
mod ux_estimate;
mod ux_ledger;
mod ux_ledger_write;
mod ux_live;
mod ux_measure;
mod ux_measure_run;
mod ux_models;
mod ux_prompt;
mod ux_report;
mod ux_request;
mod ux_review;
mod ux_run;
mod ux_secret;
mod ux_spend;
#[cfg(test)]
mod ux_stub_tests;
mod web;
mod web_version;
mod xvfb;
mod xvfb_display;

use std::process::ExitCode;

use clap::Parser;

use cli::{Cli, Command};

#[allow(clippy::too_many_lines)] // the command dispatch, one arm per command
fn main() -> ExitCode {
    let cli = Cli::parse();
    let reporter = report::Reporter::new(cli.verbose);
    let steps = match cli.command {
        Command::Setup { ci } => return setup::run(ci),
        Command::CheckTools => return setup::check(),
        Command::ApiFences { name } => return api_fences_run::run(&name),
        Command::ApiMcpInner { port } => return api_mcp_inner::run(port),
        Command::ApiBuild {
            name,
            brief,
            budget,
            model,
            allow,
        } => {
            let order = api_build_run::Order {
                name: &name,
                brief: &brief,
                budget,
                model: &model,
                allow: &allow,
            };
            return api_build_run::run(&order);
        }
        Command::ApiScoped { command } => return api_limits::scoped(&command),
        Command::ApiRelayInner {
            port,
            socket,
            command,
        } => return api_inner::run(port, &socket, &command),
        Command::CheckFiles => return files::run(),
        Command::Quality {
            distributions,
            list,
        } => return quality::run(distributions, list),
        Command::CheckLayers => return layers::run(),
        Command::CheckBuilds => return builds::run(),
        Command::CheckImports => return imports::run(),
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
        Command::Serve { replace } => return serve::run(replace),
        Command::StampPage => return stamp_page::run(),
        Command::UxReview {
            dry_run,
            pr,
            measure_image_cost,
        } => return ux_review::run(dry_run, pr, measure_image_cost),
        Command::CheckXtaskTwin { sibling } => return twin::run(&sibling),
        Command::Dev { keys, replace } => return dev::run(&reporter, &keys, replace),
        Command::Ci { except } => {
            let steps =
                web::web_package().and_then(|package| steps::without(steps::ci(&package), &except));
            match steps {
                Ok(steps) => return ci_shared::run(&steps, &reporter),
                Err(message) => return fail("ci", &message),
            }
        }
        Command::Wasm => match web::web_package() {
            Ok(package) => steps::wasm_lint(&package),
            Err(message) => return fail("wasm", &message),
        },
        Command::Gate { pr: None } => return gate::run(&reporter),
        Command::Gate { pr: Some(number) } => return gate_pr_real::run(number),
        Command::Queue { dry_run } => return queue::run(dry_run),
        Command::Scaling => return scaling::run(&reporter),
        Command::Quiet { args } => return quiet_command::run(&args),
        Command::Lint => steps::lint(),
        Command::Deny => steps::deny_check(),
        Command::Test { tier } => return test_shared::run(tier, &reporter),
    };
    ci::run(&steps, &reporter)
}

fn fail(command: &str, message: &str) -> ExitCode {
    println!("FAIL {command}: {message}");
    ExitCode::FAILURE
}
