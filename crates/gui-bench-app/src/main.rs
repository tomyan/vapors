//! vapors under gui-bench: one window per run that shows the scenario the
//! command line names (`docs/decisions/2026-11-19-gui-bench.md`). Slice
//! s1 builds the window, the sidebar, the top bar and the metric cards;
//! the rest follows in s2 and s3 (see `FLAGS.md` for parts built by hand).

mod app;
#[allow(clippy::all, clippy::pedantic, dead_code)] // vendored unchanged; docs/gui-bench/spec/NOTICE.md
#[path = "../../../docs/gui-bench/spec/cli.rs"]
mod cli;
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
