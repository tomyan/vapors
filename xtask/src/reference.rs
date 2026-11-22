//! `cargo xtask reference <fixture> <scale>`: a paint fixture's saved
//! Chromium reference captured again with the pinned Chromium (saved
//! references R1, `docs/plans/saved-chromium-references.md`), by the
//! corpus's `paint_reference` example.

use crate::step::Step;

/// The step that captures `fixture` at `scale`.
pub fn steps(fixture: &str, scale: u32) -> Vec<Step> {
    let scale = scale.to_string();
    vec![Step::new(
        "reference",
        "cargo",
        &[
            "run",
            "-p",
            "vapors-elements-corpus",
            "--example",
            "paint_reference",
            "--",
            fixture,
            &scale,
        ],
        "install the pinned Chromium with `cargo xtask setup`, unset VAPORS_CHROMIUM, and check the fixture's name",
    )]
}

#[cfg(test)]
#[path = "reference_tests.rs"]
mod tests;
