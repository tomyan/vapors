//! The live switch fixture's `.events` script played on vapors and
//! compared step by step with what Chromium logged for it
//! (`fixtures/live_switch.events.json`, from the events oracle): a click,
//! Space, Enter and Tab on a `switch` input, as on a checkbox.
//! Reproduce: `cargo test -p vapors-elements-corpus --test events_vapors_switch`.

use vapors_elements::Viewport;
use vapors_elements_corpus::events_compare::compare;
use vapors_elements_corpus::events_script::parse;
use vapors_elements_corpus::events_vapors::play;

#[path = "corpus_support/events.rs"]
mod events;

use events::{fixture, read};

/// The fixture's comment says 800x100 at 1x.
const VIEWPORT: Viewport = Viewport {
    width: 800.0,
    height: 100.0,
    scale: 1.0,
};

fn ours(script: &str) -> Result<(), String> {
    let script = parse("live_switch", script).expect("parses");
    let answers = serde_json::from_str(&read("live_switch.events.json")).expect("json");
    let log = play(&fixture("live_switch"), &script, VIEWPORT)?;
    compare("live_switch", &script, &log, &answers)
}

#[test]
fn the_live_switch_script_logs_on_vapors_what_chromium_logged() {
    // Given, When, Then: the events, focus and checked state after each step
    ours(&read("live_switch.events")).unwrap_or_else(|e| panic!("{e}"));
}

#[test]
fn a_deliberate_mismatch_fails_the_comparison() {
    // Given: the script with Enter made a Space, which toggles on vapors as
    // on Chromium, where Chromium's recorded Enter did not
    let script = read("live_switch.events");
    let wrong = script.replace("key Enter ", "key Space ");
    assert_ne!(wrong, script, "the replacement applied");

    // When, Then
    assert!(ours(&wrong).is_err(), "a different action must fail");
}
