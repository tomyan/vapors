//! `cargo xtask reference all | <fixture> <scale> [<state>]`: saved
//! Chromium references for the paint tests captured on request
//! (`docs/plans/saved-chromium-references.md`), by the corpus's
//! `paint_reference` example.

use crate::step::Step;

/// The step that captures `target`, passed to the example as it is.
pub fn steps(target: &[String]) -> Vec<Step> {
    let mut args = vec![
        "run",
        "-p",
        "vapors-elements-corpus",
        "--example",
        "paint_reference",
        "--",
    ];
    args.extend(target.iter().map(String::as_str));
    vec![Step::new(
        "reference",
        "cargo",
        &args,
        "install Chromium with `cargo xtask setup`, and name `all` or a paint fixture, scale and state",
    )]
}

#[cfg(test)]
#[path = "reference_tests.rs"]
mod tests;
