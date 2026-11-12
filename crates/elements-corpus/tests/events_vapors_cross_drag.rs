//! The cross-drag fixture's script played on vapors, its log compared
//! with Chromium's (`fixtures/cross_drag.events.json`): an uncaptured
//! mouse drag from one element to another gives the second the moves
//! and the release, and the click goes to the nearest common ancestor
//! (3D 1c PR 5; `elements_app_surface_capture.rs` cites it).

use vapors_elements_corpus::events_compare::compare;
use vapors_elements_corpus::events_script::parse;
use vapors_elements_corpus::events_vapors::play;

#[path = "corpus_support/events.rs"]
mod events;

use events::{VIEWPORT, fixture, read};

#[test]
fn an_uncaptured_cross_element_drag_logs_on_vapors_what_chromium_logged() {
    // Given
    let script = parse("cross_drag", &read("cross_drag.events")).expect("parses");
    let answers = serde_json::from_str(&read("cross_drag.events.json")).expect("json");

    // When
    let ours = play(&fixture("cross_drag"), &script, VIEWPORT).unwrap_or_else(|e| panic!("{e}"));

    // Then
    compare("cross_drag", &script, &ours, &answers).unwrap_or_else(|e| panic!("{e}"));
}
