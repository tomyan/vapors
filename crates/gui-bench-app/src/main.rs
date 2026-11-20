//! vapors under gui-bench: one window per run that shows the scenario the
//! command line names (`docs/decisions/2026-11-19-gui-bench.md`). Slice
//! s1 builds the window and the sidebar (s1b the top bar and the cards, s2
//! the chart and the filters, s2b the orders); the rest follows in s3 and s3b (see
//! `FLAGS.md` for parts built by hand).

#![allow(
    clippy::needless_pass_by_value,
    reason = "a generated mount takes its props by value, and the compiler allows that lint only when every prop is a signal or memo: the Controls prop is Copy but not one (crates/compiler/src/codegen/allowances.rs)"
)]

mod app;
#[allow(clippy::all, clippy::pedantic, dead_code)] // vendored unchanged; docs/gui-bench/spec/NOTICE.md
#[path = "../../../docs/gui-bench/spec/cli.rs"]
mod cli;
mod controls;
#[allow(clippy::all, clippy::pedantic, dead_code)] // vendored unchanged; docs/gui-bench/spec/NOTICE.md
#[path = "../../../docs/gui-bench/spec/dashboard.rs"]
mod dashboard;
include!(concat!(env!("OUT_DIR"), "/wisp.rs"));
#[cfg(test)]
mod spec_tests;
#[cfg(test)]
mod tests;

fn main() {
    let scenario = cli::from_env("vapors-bench");
    if let Err(message) = app::run(scenario) {
        eprintln!("vapors-bench: {message}");
        std::process::exit(2);
    }
}
