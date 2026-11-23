//! The committed Chromium answers match the fixtures one for one, with no
//! browser: each fixture laid out from its `.wisp` has its boxes
//! (`<name>.boxes.json`) at each viewport its `/* viewport */` comment
//! names, and its accessibility tree (`<name>.ax.json`); each `.events`
//! script has its answers (`<name>.events.json`); and no answer is left
//! that no fixture or script needs. What the answers say is held to
//! vapors by `corpus.rs` and `events_every_script.rs`, and to Chromium
//! by the oracles' `--check` runs.
//!
//! Reproduce: `cargo test -p vapors-elements-corpus --test committed_references`.

use serde_json::Value;
use vapors_elements_corpus::answer_files::{orphan, unmatched};
use vapors_elements_corpus::events_oracle::scripted;
use vapors_elements_corpus::oracle::{fixtures_dir, laid_out_fixture_names};
use vapors_elements_corpus::page::split;

/// The missing and orphaned answers with `suffix`, each naming the file
/// and the fix.
fn mismatches(wanted: &[String], suffix: &str, oracle: &str) -> Vec<String> {
    let found = unmatched(wanted, suffix).unwrap_or_else(|e| panic!("{e}"));
    let missing = found.missing.iter().map(|name| {
        format!(
            "fixtures/{name}{suffix} is missing; fix: cargo run -p vapors-elements-corpus --example {oracle}, review and commit it"
        )
    });
    let orphaned = found.orphaned.iter().map(|name| orphan(name, suffix));
    missing.chain(orphaned).collect()
}

/// The (width, height, scale) of each viewport in fixture `name`'s
/// committed boxes, in order.
fn committed_viewports(name: &str) -> Vec<(f64, f64, f64)> {
    let path = fixtures_dir().join(format!("{name}.boxes.json"));
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let json: Value =
        serde_json::from_str(&text).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let number = |v: &Value| {
        v.as_f64()
            .unwrap_or_else(|| panic!("{}: a viewport of another shape: {v}", path.display()))
    };
    json["viewports"]
        .as_array()
        .unwrap_or_else(|| panic!("{}: no viewports", path.display()))
        .iter()
        .map(|s| {
            (
                number(&s["viewport"][0]),
                number(&s["viewport"][1]),
                number(&s["scale"]),
            )
        })
        .collect()
}

/// Fixture `name`'s boxes, if they are not at the viewports its `.wisp` names.
fn viewports_differ(name: &str) -> Option<String> {
    let path = fixtures_dir().join(format!("{name}.wisp"));
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let source = split(name, &text).unwrap_or_else(|e| panic!("{e}"));
    let wanted: Vec<(f64, f64, f64)> = source
        .viewports
        .iter()
        .map(|v| (f64::from(v.width), f64::from(v.height), v.scale))
        .collect();
    let committed = committed_viewports(name);
    (committed != wanted).then(|| {
        format!(
            "fixtures/{name}.boxes.json is at {committed:?}, {name}.wisp names {wanted:?} (width, height, scale); fix: cargo run -p vapors-elements-corpus --example layout_oracle, review and commit it"
        )
    })
}

#[test]
fn every_answer_matches_a_fixture_one_for_one() {
    // Given
    let laid_out = laid_out_fixture_names().unwrap_or_else(|e| panic!("{e}"));
    let scripts = scripted().unwrap_or_else(|e| panic!("{e}"));
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

#[test]
fn every_fixtures_boxes_are_at_the_viewports_it_names() {
    // Given
    let laid_out = laid_out_fixture_names().unwrap_or_else(|e| panic!("{e}"));
    let have_boxes = unmatched(&laid_out, ".boxes.json").unwrap_or_else(|e| panic!("{e}"));

    // When: a missing file is the other test's failure
    let found: Vec<String> = laid_out
        .iter()
        .filter(|name| !have_boxes.missing.contains(name))
        .filter_map(|name| viewports_differ(name))
        .collect();

    // Then
    assert!(found.is_empty(), "{}", found.join("\n"));
}
