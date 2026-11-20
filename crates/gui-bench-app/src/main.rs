//! vapors under gui-bench: one window per run that shows the scenario the
//! command line names (`docs/decisions/2026-11-19-gui-bench.md`). Slice
//! s1 builds the window and the sidebar (s1b the top bar and the cards, s2
//! the chart and the filters, s2b the orders); the rest follows in s3b (see
//! `FLAGS.md` for parts built by hand).

#![allow(
    clippy::missing_panics_doc,
    reason = "a generated mount of MessageCard panics on a colour outside AVATAR_COLORS, as it must, and the generated function has no doc to put `# Panics` in"
)]

mod app;
#[allow(clippy::all, clippy::pedantic, dead_code)] // vendored unchanged; docs/gui-bench/spec/NOTICE.md
#[path = "../../../docs/gui-bench/spec/cli.rs"]
mod cli;
mod controls;
#[allow(clippy::all, clippy::pedantic, dead_code)] // vendored unchanged; docs/gui-bench/spec/NOTICE.md
#[path = "../../../docs/gui-bench/spec/dashboard.rs"]
mod dashboard;
mod order_parts;
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
