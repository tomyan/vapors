//! Every fixture's `.events` script played on vapors and compared, step
//! by step, with Chromium's committed answers (`<name>.events.json`,
//! written by the events oracle): no browser runs here. What each script
//! covers is in its header comment. A script with no committed answers
//! fails, naming the file and the oracle command.
//!
//! Reproduce: `cargo test -p vapors-elements-corpus --test events_every_script`.

use vapors_elements::Viewport;
use vapors_elements_corpus::events_compare::compare;
use vapors_elements_corpus::events_oracle::{PAGE_SIZE, scripted};
use vapors_elements_corpus::events_script::{Script, parse};
use vapors_elements_corpus::events_vapors::play;
use vapors_elements_corpus::page::split;

#[path = "corpus_support/events.rs"]
#[expect(
    dead_code,
    reason = "the viewport comes from each fixture (`VIEWPORT`)"
)]
mod events;

use events::{fixture, read};

/// A relaxed guard: scripts the vapors player cannot play yet, each with
/// the word its refusal names. Their answers are still checked against
/// Chromium by `cargo run -p vapors-elements-corpus --example
/// events_oracle -- --check`, not against vapors. An entry is retired
/// when the player plays its script: the test then fails until it is
/// removed here.
const NOT_PLAYED_ON_VAPORS: [(&str, &str); 1] = [("buttons", "never fires `wheel`")];

/// The size the events oracle opens the page at: the script's own (`at`),
/// else for another page (`page`) `PAGE_SIZE` at 1x, else the fixture's
/// first viewport.
#[allow(clippy::cast_possible_truncation)] // a device scale is 1 or 2
#[allow(clippy::cast_precision_loss)] // viewports are a few hundred pixels
fn viewport(name: &str, script: &Script) -> Viewport {
    let (size, scale) = if script.page.is_some() {
        (PAGE_SIZE, 1.0)
    } else {
        let source = split(name, &read(&format!("{name}.wisp"))).unwrap_or_else(|e| panic!("{e}"));
        let first = source.viewports[0];
        ((first.width, first.height), first.scale)
    };
    let (width, height) = script.viewport.unwrap_or(size);
    Viewport {
        width: width as f32,
        height: height as f32,
        scale: scale as f32,
    }
}

/// Fixture `name`'s script on vapors against Chromium's answers.
fn played(name: &str) -> Result<(), String> {
    let script = parse(name, &read(&format!("{name}.events")))?;
    let answers = committed(name)?;
    let log = play(&fixture(name), &script, viewport(name, &script))?;
    compare(name, &script, &log, &answers)
}

fn committed(name: &str) -> Result<serde_json::Value, String> {
    let path = vapors_elements_corpus::oracle::fixtures_dir().join(format!("{name}.events.json"));
    let text = std::fs::read_to_string(&path).map_err(|e| {
        format!(
            "{}: {e}; fix: cargo run -p vapors-elements-corpus --example events_oracle, review and commit it",
            path.display()
        )
    })?;
    serde_json::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))
}

/// What a script's run says that it should not, if anything.
fn judged(name: &str) -> Option<String> {
    let not_played = NOT_PLAYED_ON_VAPORS.iter().find(|(n, _)| *n == name);
    match (played(name), not_played) {
        (Ok(()), None) => None,
        (Err(e), None) => Some(e),
        (Err(e), Some((_, word))) if e.contains(word) => None,
        (Err(e), Some((_, word))) => Some(format!(
            "{name}: listed in NOT_PLAYED_ON_VAPORS for {word}, but refused otherwise: {e}"
        )),
        (Ok(()), Some(_)) => Some(format!(
            "{name}: the vapors player plays it now; fix: remove it from NOT_PLAYED_ON_VAPORS"
        )),
    }
}

#[test]
fn every_script_logs_on_vapors_what_chromium_logged() {
    // Given
    let names = scripted().unwrap_or_else(|e| panic!("{e}"));
    assert!(!names.is_empty(), "no .events scripts: nothing is compared");

    // When
    let failures: Vec<String> = names.iter().filter_map(|name| judged(name)).collect();

    // Then
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn every_script_the_player_cannot_play_is_one_of_the_fixtures() {
    // Given
    let names = scripted().unwrap_or_else(|e| panic!("{e}"));

    // Then: a stale entry would excuse nothing and hide nothing
    for (name, _) in NOT_PLAYED_ON_VAPORS {
        assert!(
            names.iter().any(|n| n == name),
            "{name} in NOT_PLAYED_ON_VAPORS has no fixtures/{name}.events; fix: remove it"
        );
    }
}
