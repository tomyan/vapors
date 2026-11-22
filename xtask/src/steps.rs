//! The steps the factory knows how to run, and which commands use them.
//!
//! Order matters: `ci()` runs the fastest checks first so most failures
//! show within the first minute.

pub use crate::step::Step;
use crate::steps_checks::{
    browser_profile, builds, files, imports, layers, nightly, nightly_issues,
};

#[path = "steps_tiers.rs"]
mod tiers;
use tiers::{gpu, host, pmu, unit};

fn tools() -> Step {
    Step::new(
        "tools",
        "cargo",
        &["xtask", "check-tools"],
        "run `cargo xtask setup`",
    )
}

/// The variable that turns off cargo deny's fetching: set to `1` by the
/// API builder's sandbox, which has no network and binds the advisory
/// database its host fetched, read-only (`docs/plans/api-builder.md`,
/// slice 4; the lead, L43). Every check still runs, on that database.
/// The gate refuses to run while it is set (`refuse_deny_offline`), so a
/// merge always checks a database fetched that run. Relaxed guard,
/// declared: inside the sandbox the database is as fresh as the host's
/// fetch just before (`request_push`); retired when the sandbox can reach
/// the advisory database itself.
pub const DENY_OFFLINE: &str = "VAPORS_DENY_OFFLINE";

/// Refuses [`DENY_OFFLINE`] set at all, naming it: for the gate.
///
/// # Errors
///
/// The variable set, with its value.
#[allow(
    clippy::unnecessary_debug_formatting,
    reason = "Debug quotes the value and escapes its control and non-UTF-8 bytes"
)]
pub fn refuse_deny_offline(value: Option<&std::ffi::OsStr>) -> Result<(), String> {
    match value {
        None => Ok(()),
        Some(value) => Err(format!(
            "{DENY_OFFLINE}={value:?} is set: the gate's deny step must fetch the advisory database; fix: unset {DENY_OFFLINE}"
        )),
    }
}

fn deny() -> Step {
    deny_with(std::env::var_os(DENY_OFFLINE).as_deref())
}

/// cargo-deny's advisories, bans, licences and sources, by deny.toml,
/// for [`DENY_OFFLINE`]'s value: unset fetches the advisory database
/// every run and fails when it cannot, with a database cached or not;
/// `1` reads the one on disk (`--offline`); any other value, a non-UTF-8
/// one included, is refused by a step that fails naming it. A cold fetch
/// and check took 1.2 to 1.4 s here (2026-10-23), and `--offline` 1.5 s
/// (2026-11-17, mostly cargo's start); 60 s leaves room for a slow
/// network before the fetch is declared hung.
fn deny_with(offline: Option<&std::ffi::OsStr>) -> Step {
    let args: &[&str] = match offline.map(|value| value.to_str()) {
        None => &["deny", "check"],
        Some(Some("1")) => &["deny", "--offline", "check"],
        Some(_) => return refused_deny(offline.unwrap_or_default()),
    };
    let mut step = Step::new(
        "deny",
        "cargo",
        args,
        "fix the dependency, or record a reasoned exception in deny.toml; if the advisory database could not be fetched (offline), connect and rerun `cargo xtask deny`: it is never skipped",
    );
    step.deadline = Some(std::time::Duration::from_secs(60));
    step
}

/// A step that prints why it refuses and exits 1, its fix naming the
/// value refused.
#[allow(
    clippy::unnecessary_debug_formatting,
    reason = "Debug quotes the value and escapes its control and non-UTF-8 bytes"
)]
fn refused_deny(value: &std::ffi::OsStr) -> Step {
    let said = format!("{DENY_OFFLINE}={value:?} is neither unset nor 1");
    let fix = format!("{said}: unset it, or set it to 1 where the advisory database is bound");
    Step::new(
        "deny",
        "sh",
        &["-c", "printf '%s\\n' \"$1\" >&2; exit 1", "deny", &said],
        &fix,
    )
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

/// `steps` without those named in `except`, each skip printed: for a
/// caller that cannot run them and declares why at its call
/// (`api_push.rs`).
///
/// # Errors
///
/// A name no step has, naming the steps there are.
pub fn without(steps: Vec<Step>, except: &[String]) -> Result<Vec<Step>, String> {
    if let Some(unknown) = except
        .iter()
        .find(|name| steps.iter().all(|step| &step.name != *name))
    {
        let names: Vec<&str> = steps.iter().map(|step| step.name.as_str()).collect();
        return Err(format!(
            "--except {unknown}: ci has no such step ({})",
            names.join(", ")
        ));
    }
    for name in except {
        println!("skip {name}: --except, declared by the caller");
    }
    Ok(steps
        .into_iter()
        .filter(|step| !except.contains(&step.name))
        .collect())
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
        nightly(),
        nightly_issues(),
        browser_profile(),
        deny(),
        clippy(),
        wasm(web_package),
        wasm_release(web_package),
        unit(),
        host(),
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
    vec![
        fmt(),
        files(),
        layers(),
        builds(),
        imports(),
        nightly(),
        nightly_issues(),
        browser_profile(),
        clippy(),
    ]
}

/// The unit tier.
pub fn unit_tests() -> Vec<Step> {
    vec![unit()]
}

/// The GPU tier.
pub fn gpu_tests() -> Vec<Step> {
    vec![gpu()]
}

/// The host tier: what only the host has (`steps_tiers.rs` `HOST_TESTS`).
pub fn host_tests() -> Vec<Step> {
    vec![host()]
}

/// The hardware-counter tier: never part of `ci`.
pub fn pmu_tests() -> Vec<Step> {
    vec![pmu()]
}

#[cfg(test)]
#[path = "steps_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "steps_nextest_tests.rs"]
mod nextest_tests;
