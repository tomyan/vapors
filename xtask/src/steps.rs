//! The steps the factory knows how to run, and which commands use them.
//!
//! Order matters: `ci()` runs the fastest checks first so most failures
//! show within the first minute.

pub use crate::step::Step;

fn tools() -> Step {
    Step::new(
        "tools",
        "cargo",
        &["xtask", "check-tools"],
        "run `cargo xtask setup`",
    )
}

/// Every source file within `[files] max_lines`; `cargo xtask check-files`.
fn files() -> Step {
    Step::new(
        "files",
        "cargo",
        &["xtask", "check-files"],
        "split the file, or allow it in .config/factory.toml under [files.allow] with a reason",
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
    Step::new(
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
    )
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
        clippy(),
        wasm(web_package),
        unit(),
        gpu(),
    ]
}

/// The wasm lint alone.
pub fn wasm_lint(web_package: &str) -> Vec<Step> {
    vec![wasm(web_package)]
}

/// Formatting and lints only.
pub fn lint() -> Vec<Step> {
    vec![fmt(), files(), clippy()]
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
mod tests {
    use super::*;

    #[test]
    fn ci_runs_fastest_checks_first() {
        // Given
        let steps = ci("vapors-web-demo");

        // When
        let names: Vec<&str> = steps.iter().map(|s| s.name.as_str()).collect();

        // Then
        assert_eq!(
            names,
            ["tools", "fmt", "files", "clippy", "wasm", "unit", "gpu"]
        );
    }

    #[test]
    fn reproduce_line_is_the_pasteable_command() {
        // Given
        let step = fmt();

        // When
        let line = step.reproduce();

        // Then
        assert_eq!(line, "cargo fmt --all --check");
    }

    #[test]
    fn wasm_step_lints_the_configured_package_for_the_browser_target() {
        // Given
        let step = wasm("vapors-web-demo");

        // When
        let line = step.reproduce();

        // Then
        assert_eq!(
            line,
            "cargo clippy --target wasm32-unknown-unknown -p vapors-web-demo -- -D warnings"
        );
    }
}

#[cfg(test)]
#[path = "steps_nextest_tests.rs"]
mod nextest_tests;
