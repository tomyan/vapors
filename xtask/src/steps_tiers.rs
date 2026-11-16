//! The two test tiers, split on the test function's name.

use super::Step;

// GPU tests are the tests GPU_TESTS selects by nextest id: a function
// named `gpu_`, and in a crate whose integration tests are one binary
// every test in a `tests/gpu_*.rs` file, since the file is the id's
// first segment. The tiers split on the id because nextest refuses a
// `binary()` filter that matches no binary, and a repository may have
// no GPU tests yet. The unit tier sets this variable and
// gpu_harness::gpu() refuses it, so a GPU test the filter does not
// select fails here on a machine with a GPU, as it would on a runner
// without. A test in a `gpu_*.rs` file that opens no GPU runs in the
// gpu tier only, unchecked by that refusal.
pub(super) const UNIT_TIER: (&str, &str) = ("VAPORS_TEST_TIER", "unit");

/// The GPU tests: a nextest id beginning `gpu_` (a test in a root `gpu_`
/// module, or a whole crate binary's file named so), or one whose
/// function name begins `gpu_` (a crate whose integration tests are one
/// binary, `scripts/one-test-binary.sh`, has ids `<file>::<function>`).
pub(super) const GPU_TESTS: &str = "test(/^gpu_/) | test(/::gpu_[^:]*$/)";

pub(super) fn unit() -> Step {
    let mut step = Step::new(
        "unit",
        "cargo",
        &[
            "nextest",
            "run",
            "--workspace",
            "-E",
            &format!("not ({GPU_TESTS})"),
        ],
        "run the failing test alone with `cargo nextest run <name>`",
    );
    step.env
        .push((UNIT_TIER.0.to_owned(), UNIT_TIER.1.to_owned()));
    step
}

pub(super) fn gpu() -> Step {
    Step::new(
        "gpu",
        "cargo",
        &["nextest", "run", "--workspace", "-E", GPU_TESTS],
        "needs a Vulkan adapter; the failure names what was found and how to select one",
    )
}

#[cfg(test)]
#[path = "steps_tiers_tests.rs"]
mod tests;
