//! The steps the factory knows how to run, and which commands use them.
//!
//! Order matters: `ci()` runs the fastest checks first so most failures
//! show within the first minute.

pub use crate::step::Step;
use crate::steps_checks::{builds, files, imports, layers};

fn tools() -> Step {
    Step::new(
        "tools",
        "cargo",
        &["xtask", "check-tools"],
        "run `cargo xtask setup`",
    )
}

/// cargo-deny's advisories, bans, licences and sources, by deny.toml.
/// It fetches the advisory database every run and fails when it cannot,
/// with a database cached or not; xtask never passes --disable-fetch, so
/// the check is never skipped. A cold fetch and check took 1.2 to 1.4 s
/// here (2026-10-23); 60 s leaves room for a slow network before the
/// fetch is declared hung.
fn deny() -> Step {
    let mut step = Step::new(
        "deny",
        "cargo",
        &["deny", "check"],
        "fix the dependency, or record a reasoned exception in deny.toml; if the advisory database could not be fetched (offline), connect and rerun `cargo xtask deny`: it is never skipped",
    );
    step.deadline = Some(std::time::Duration::from_secs(60));
    step
}

fn fmt() -> Step {
    Step::new(
        "fmt",
        "cargo",
        &["fmt", "--all", "--check"],
        "run `cargo fmt --all`",
    )
}

fn clippy() -> Step {
    Step::new(
        "clippy",
        "cargo",
        &[
            "clippy",
            "--workspace",
            "--all-targets",
            "--",
            "-D",
            "warnings",
        ],
        "fix the lint, or allow it at the site with a comment saying why",
    )
}

/// The web package and everything under it, linted for the browser
/// target, so a native-only `use` in a shared crate fails here and not
/// in the trunk build.
fn wasm(web_package: &str) -> Step {
    denying_warnings(Step::new(
        "wasm",
        "cargo",
        &[
            "clippy",
            "--target",
            "wasm32-unknown-unknown",
            "-p",
            web_package,
            "--",
            "-D",
            "warnings",
        ],
        "put the native-only code under cfg(not(target_family = \"wasm\")) and give the web its own",
    ))
}

/// `step` with `RUSTFLAGS=-D warnings`, as CI sets it: clippy's own
/// `-- -D warnings` covers only the package named, and a warning in a
/// dependency (runtime, engine, document, viewer) would pass.
fn denying_warnings(mut step: Step) -> Step {
    step.env
        .push(("RUSTFLAGS".to_owned(), "-D warnings".to_owned()));
    step
}

/// The web package linted as trunk builds it, in release: code under
/// `cfg(debug_assertions)` is gone there, so a binding only debug code
/// reads is unused (#34's `stub_scenario`, which passed `wasm` and failed
/// the page build).
fn wasm_release(web_package: &str) -> Step {
    denying_warnings(Step::new(
        "wasm release",
        "cargo",
        &[
            "clippy",
            "--target",
            "wasm32-unknown-unknown",
            "-p",
            web_package,
            "--release",
            "--",
            "-D",
            "warnings",
        ],
        "a binding or import only debug builds use: cfg it out in release, or use it there",
    ))
}

// GPU tests are test functions whose names start with `gpu_` (kept in
// `tests/gpu_*.rs` files); the tiers split on the function name because
// nextest refuses a `binary()` filter that matches no binary, and a
// repository may have no GPU tests yet.
fn unit() -> Step {
    Step::new(
        "unit",
        "cargo",
        &["nextest", "run", "--workspace", "-E", "not test(/^gpu_/)"],
        "run the failing test alone with `cargo nextest run <name>`",
    )
}

fn gpu() -> Step {
    Step::new(
        "gpu",
        "cargo",
        &["nextest", "run", "--workspace", "-E", "test(/^gpu_/)"],
        "needs a Vulkan adapter; the failure names what was found and how to select one",
    )
}

/// Everything CI runs, fastest first. `web_package` is `web.package`
/// from `.config/factory.toml`.
pub fn ci(web_package: &str) -> Vec<Step> {
    vec![
        tools(),
        fmt(),
        files(),
        layers(),
        builds(),
        imports(),
        deny(),
        clippy(),
        wasm(web_package),
        wasm_release(web_package),
        unit(),
        gpu(),
    ]
}

/// The wasm lints alone, debug and release.
pub fn wasm_lint(web_package: &str) -> Vec<Step> {
    vec![wasm(web_package), wasm_release(web_package)]
}

/// cargo-deny alone.
pub fn deny_check() -> Vec<Step> {
    vec![deny()]
}

/// Formatting and lints only.
pub fn lint() -> Vec<Step> {
    vec![fmt(), files(), layers(), builds(), imports(), clippy()]
}

/// The unit tier.
pub fn unit_tests() -> Vec<Step> {
    vec![unit()]
}

/// The GPU tier.
pub fn gpu_tests() -> Vec<Step> {
    vec![gpu()]
}

#[cfg(test)]
#[path = "steps_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "steps_nextest_tests.rs"]
mod nextest_tests;
