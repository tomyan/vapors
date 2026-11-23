//! A changed live switch script fails the comparison with Chromium's
//! answers (`fixtures/live_switch.events.json`): the script itself is
//! played in `events_every_script.rs`.
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
fn a_deliberate_mismatch_fails_the_comparison() {
    // Given: the script with Enter made a Space, which toggles on vapors as
    // on Chromium, where Chromium's recorded Enter did not
    let script = read("live_switch.events");
    let wrong = script.replace("key Enter ", "key Space ");
    assert_ne!(wrong, script, "the replacement applied");

    // When, Then
    assert!(ours(&wrong).is_err(), "a different action must fail");
}
