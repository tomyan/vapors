//! Chromium's side of a paint test: a committed saved reference for the
//! fixtures and scales [`SAVED`] lists, read without launching a browser
//! (`vapors_elements_corpus::paint_reference`); a live screenshot for
//! every other. Saved references R1, `docs/plans/saved-chromium-references.md`.

use vapors_text::comparison::Rgba;

use super::paint_support::chromiums;

/// The fixtures and scales whose Chromium side is a saved reference.
const SAVED: &[(&str, u32)] = &[("text_line_box_paint", 2)];

/// Chromium's rendering of fixture `name` at `scale`: its saved
/// reference when [`SAVED`] lists it, refused when that is missing or
/// stale; else a live screenshot for test `test` ([`chromiums`]).
pub fn theirs(test: &str, name: &str, scale: u32) -> Rgba {
    if SAVED.contains(&(name, scale)) {
        return vapors_elements_corpus::paint_reference::saved(name, scale)
            .unwrap_or_else(|why| panic!("{why}"));
    }
    chromiums(test, name, scale, "")
}
