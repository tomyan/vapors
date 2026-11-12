//! The two test tiers, split on the test function's name.

use super::Step;

// GPU tests are test functions whose names start with `gpu_` (kept in
// `tests/gpu_*.rs` files); the tiers split on the function name because
// nextest refuses a `binary()` filter that matches no binary, and a
// repository may have no GPU tests yet. The unit tier sets this variable
// and gpu_harness::gpu() refuses it, so a GPU test without the prefix
// fails here on a machine with a GPU, as it would on a runner without.
pub(super) const UNIT_TIER: (&str, &str) = ("VAPORS_TEST_TIER", "unit");

pub(super) fn unit() -> Step {
    let mut step = Step::new(
        "unit",
        "cargo",
        &["nextest", "run", "--workspace", "-E", "not test(/^gpu_/)"],
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
        &["nextest", "run", "--workspace", "-E", "test(/^gpu_/)"],
        "needs a Vulkan adapter; the failure names what was found and how to select one",
    )
}

#[cfg(test)]
#[path = "steps_tiers_tests.rs"]
mod tests;
