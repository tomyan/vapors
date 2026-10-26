//! The capture fixture's `.events` script played on vapors, its log
//! compared step by step with what Chromium logged for the same script
//! (`fixtures/capture.events.json`, written by the events oracle): the
//! order and targets of pointer capture's events (3-E1).

use vapors_elements::Viewport;
use vapors_elements_corpus::events_compare::compare;
use vapors_elements_corpus::events_script::parse;
use vapors_elements_corpus::events_vapors::play;
use vapors_elements_corpus::oracle::fixtures_dir;
use vapors_elements_corpus::{Fixture, all};

/// The fixture's comment says 640x400 at 1x.
const VIEWPORT: Viewport = Viewport {
    width: 640.0,
    height: 400.0,
    scale: 1.0,
};

fn fixture(name: &str) -> Fixture {
    all()
        .into_iter()
        .find(|f| f.name == name)
        .expect("a fixture")
}

fn read(file: &str) -> String {
    let path = fixtures_dir().join(file);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

#[test]
fn the_capture_script_logs_on_vapors_what_chromium_logged() {
    // Given
    let script = parse("capture", &read("capture.events")).expect("parses");
    let answers = serde_json::from_str(&read("capture.events.json")).expect("json");

    // When
    let ours = play(&fixture("capture"), &script, VIEWPORT).unwrap_or_else(|e| panic!("{e}"));

    // Then
    compare("capture", &script, &ours, &answers).unwrap_or_else(|e| panic!("{e}"));
}

#[test]
fn an_action_not_played_on_vapors_is_refused_naming_its_line() {
    // Given
    let script = parse(
        "capture",
        "listen click\nclick 5 5\nexpect event click div#thumb\nkey Tab\nexpect no event click\n",
    )
    .expect("parses");

    // When
    let refusal = play(&fixture("capture"), &script, VIEWPORT).expect_err("refused");

    // Then
    assert!(
        refusal.contains("capture.events:4:") && refusal.contains("Key"),
        "{refusal}"
    );
}

#[test]
fn the_players_tap_slop_is_the_platforms() {
    // Then: one number, in two crates
    assert_eq!(
        vapors_elements_corpus::events_vapors::TAP_SLOP,
        vapors_platform::TAP_SLOP
    );
}

#[test]
fn a_touch_that_stays_inside_the_slop_is_refused_as_a_tap() {
    // Given: an 8 px move, which Chromium and the platform treat as a tap
    let script = parse(
        "capture",
        "listen pointerup\ntouch-down 50 60\nexpect event pointerdown div#track\ntouch-move 50 52\nexpect no event pointerup\ntouch-up 50 52\nexpect event pointerup div#track\n",
    )
    .expect("parses");

    // When
    let refusal = play(&fixture("capture"), &script, VIEWPORT).expect_err("refused");

    // Then
    assert!(
        refusal.contains("capture.events:6:") && refusal.contains("tap"),
        "{refusal}"
    );
}
