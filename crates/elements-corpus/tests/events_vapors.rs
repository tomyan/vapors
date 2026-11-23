//! The vapors player's refusals: an action it cannot play, and a touch
//! that stays inside the tap slop. Every fixture's own script is played
//! on vapors and compared with Chromium's answers in
//! `events_every_script.rs`.

use vapors_elements_corpus::events_script::parse;
use vapors_elements_corpus::events_vapors::play;

#[path = "corpus_support/events.rs"]
#[expect(
    dead_code,
    reason = "these scripts are inline, so no fixture file is read (`read`)"
)]
mod events;

use events::{VIEWPORT, fixture};

#[test]
fn an_action_not_played_on_vapors_is_refused_naming_its_line() {
    // Given
    let script = parse(
        "capture",
        "listen click\nclick 5 5\nexpect event click div#thumb\nwheel 5 5 0 120\nexpect no event click\n",
    )
    .expect("parses");

    // When
    let refusal = play(&fixture("capture"), &script, VIEWPORT).expect_err("refused");

    // Then
    assert!(
        refusal.contains("capture.events:4:") && refusal.contains("Wheel"),
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
