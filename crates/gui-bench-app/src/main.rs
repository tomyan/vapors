//! vapors under gui-bench: one window per run that shows the scenario the
//! command line names. Built in slices s1 to s3
//! (`docs/decisions/2026-11-19-gui-bench.md`); until s1, it refuses.

// The vendored spec, compiled into the tests now so its own tests run
// before the app uses it; s1 includes it in the binary too.
#[cfg(test)]
#[allow(clippy::all, clippy::pedantic, dead_code)] // vendored unchanged; docs/gui-bench/spec/NOTICE.md
#[path = "../../../docs/gui-bench/spec/cli.rs"]
mod cli;
#[cfg(test)]
#[allow(clippy::all, clippy::pedantic, dead_code)] // vendored unchanged; docs/gui-bench/spec/NOTICE.md
#[path = "../../../docs/gui-bench/spec/dashboard.rs"]
mod dashboard;
#[cfg(test)]
mod spec_tests;

fn main() {
    eprintln!(
        "vapors-bench: not built yet; slice s1 builds the app (docs/decisions/2026-11-19-gui-bench.md)"
    );
    std::process::exit(2);
}
