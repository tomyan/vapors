//! The committed Chromium answers match the fixtures one for one, with no
//! browser: each fixture laid out from its `.wisp` has its boxes
//! (`<name>.boxes.json`) and accessibility tree (`<name>.ax.json`), each
//! `.events` script has its answers (`<name>.events.json`), and no answer
//! is left whose fixture or script was removed. The answers themselves
//! are held to vapors by `corpus.rs` and `events_every_script.rs`, and
//! to Chromium by the oracles' `--check` runs.
//!
//! Reproduce: `cargo test -p vapors-elements-corpus --test committed_references`.

use std::collections::BTreeSet;

use vapors_elements_corpus::events_oracle::scripted;
use vapors_elements_corpus::oracle::{fixtures_dir, laid_out_fixture_names};

/// The names of the files under `fixtures/` ending in `suffix`.
fn committed(suffix: &str) -> BTreeSet<String> {
    let dir = fixtures_dir();
    let entries = std::fs::read_dir(&dir).unwrap_or_else(|e| panic!("{}: {e}", dir.display()));
    entries
        .map(|entry| entry.unwrap_or_else(|e| panic!("{}: {e}", dir.display())))
        .filter_map(|entry| {
            let file = entry.file_name().to_string_lossy().into_owned();
            file.strip_suffix(suffix).map(str::to_owned)
        })
        .collect()
}

/// What differs between the names that need an answer and those that
/// have one, each naming the file and the fix.
fn mismatches(wanted: &BTreeSet<String>, suffix: &str, oracle: &str) -> Vec<String> {
    let found = committed(suffix);
    let fix = format!("cargo run -p vapors-elements-corpus --example {oracle}");
    let missing = wanted.difference(&found).map(|name| {
        format!("fixtures/{name}{suffix} is missing; fix: {fix}, review and commit it")
    });
    let orphaned = found.difference(wanted).map(|name| {
        format!("fixtures/{name}{suffix} answers no fixture that needs it; fix: remove it, or restore its fixture or script")
    });
    missing.chain(orphaned).collect()
}

#[test]
fn every_answer_matches_a_fixture_one_for_one() {
    // Given
    let laid_out: BTreeSet<String> = laid_out_fixture_names()
        .unwrap_or_else(|e| panic!("{e}"))
        .into_iter()
        .collect();
    let scripts: BTreeSet<String> = scripted()
        .unwrap_or_else(|e| panic!("{e}"))
        .into_iter()
        .collect();
    assert!(
        !laid_out.is_empty(),
        "no laid-out fixtures: nothing is compared"
    );
    assert!(
        !scripts.is_empty(),
        "no .events scripts: nothing is compared"
    );

    // When
    let found: Vec<String> = [
        mismatches(&laid_out, ".boxes.json", "layout_oracle"),
        mismatches(&laid_out, ".ax.json", "ax_oracle"),
        mismatches(&scripts, ".events.json", "events_oracle"),
    ]
    .concat();

    // Then
    assert!(found.is_empty(), "{}", found.join("\n"));
}
