//! The steps the factory knows how to run, and which commands use them.
//!
//! Order matters: `ci()` runs the fastest checks first so most failures
//! show within the first minute.

/// One external command with the advice to print when it fails.
pub struct Step {
    /// Short name shown in the report, such as `fmt`.
    pub name: &'static str,
    /// The program and its arguments, run from the workspace root.
    pub program: &'static str,
    /// Arguments for `program`.
    pub args: &'static [&'static str],
    /// What to do when this step fails.
    pub fix: &'static str,
}

impl Step {
    /// The command line a reader can paste to reproduce this step.
    pub fn reproduce(&self) -> String {
        let mut line = String::from(self.program);
        for arg in self.args {
            line.push(' ');
            line.push_str(arg);
        }
        line
    }
}

const TOOLS: Step = Step {
    name: "tools",
    program: "cargo",
    args: &["xtask", "check-tools"],
    fix: "run `cargo xtask setup`",
};

const FMT: Step = Step {
    name: "fmt",
    program: "cargo",
    args: &["fmt", "--all", "--check"],
    fix: "run `cargo fmt --all`",
};

const CLIPPY: Step = Step {
    name: "clippy",
    program: "cargo",
    args: &[
        "clippy",
        "--workspace",
        "--all-targets",
        "--",
        "-D",
        "warnings",
    ],
    fix: "fix the lint, or allow it at the site with a comment saying why",
};

// GPU tests are test functions whose names start with `gpu_` (kept in
// `tests/gpu_*.rs` files); the tiers split on the function name because
// nextest refuses a `binary()` filter that matches no binary, and a
// repository may have no GPU tests yet.
const UNIT: Step = Step {
    name: "unit",
    program: "cargo",
    args: &["nextest", "run", "--workspace", "-E", "not test(/^gpu_/)"],
    fix: "run the failing test alone with `cargo nextest run <name>`",
};

const GPU: Step = Step {
    name: "gpu",
    program: "cargo",
    args: &["nextest", "run", "--workspace", "-E", "test(/^gpu_/)"],
    fix: "needs a Vulkan adapter; the failure names what was found and how to select one",
};

/// Everything CI runs, fastest first.
pub fn ci() -> Vec<Step> {
    vec![TOOLS, FMT, CLIPPY, UNIT, GPU]
}

/// Formatting and lints only.
pub fn lint() -> Vec<Step> {
    vec![FMT, CLIPPY]
}

/// The unit tier.
pub fn unit_tests() -> Vec<Step> {
    vec![UNIT]
}

/// The GPU tier.
pub fn gpu_tests() -> Vec<Step> {
    vec![GPU]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ci_runs_fastest_checks_first() {
        // Given
        let steps = ci();

        // When
        let names: Vec<&str> = steps.iter().map(|s| s.name).collect();

        // Then
        assert_eq!(names, ["tools", "fmt", "clippy", "unit", "gpu"]);
    }

    #[test]
    fn reproduce_line_is_the_pasteable_command() {
        // Given
        let step = FMT;

        // When
        let line = step.reproduce();

        // Then
        assert_eq!(line, "cargo fmt --all --check");
    }
}
