//! GPU tier: each fling coast Chrome on Android scrolled (a transcript
//! in tests/fling/, recorded with scripts/fling-recorder on the Pixel,
//! and its start fitted to its first 12 frames beside it;
//! docs/plans/touch-momentum.md), started in vapors where and as fast
//! as Chrome's began, scrolls the list as Chrome did (`fling_checks.rs`).
//! The fit saw only the first frames: the rest of the path, the rest
//! frame and the edge are checks it never saw.

#[path = "support/fling_checks.rs"]
mod fling_checks;
#[path = "support/fling_replay.rs"]
mod fling_replay;
#[path = "support/fling_transcript.rs"]
mod fling_transcript;

use std::path::PathBuf;

use fling_checks::{
    FIRST_FRAMES, came_to_rest, not_idle, off_edge, off_path, rest_apart, rest_gap, travelled,
    worst,
};
use fling_replay::{Replayed, replay};
use fling_transcript::load;
use vapors_gpu_harness::gpu;

/// How a recorded coast ended.
#[derive(Clone, Copy, PartialEq, Debug)]
enum Ending {
    /// The list came to rest (or to its end) and held still.
    Rest,
    /// The next touch cut the coast while it still moved 1 to 3 px a
    /// frame: it has no rest frame to hold vapors to.
    Cut,
}

const FIXTURES: &[(&str, Ending)] = &[
    ("fling-003", Ending::Cut),
    ("fling-005", Ending::Cut),
    ("fling-006", Ending::Rest),
    ("fling-010", Ending::Rest),
    ("fling-016", Ending::Rest),
    ("fling-019", Ending::Rest),
    ("fling-021", Ending::Rest),
    ("fling-022", Ending::Cut),
    ("fling-023", Ending::Rest),
];

fn dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fling")
}

/// The transcripts in tests/fling (not their `.coast.json`), sorted.
fn transcripts() -> Vec<String> {
    let dir = dir();
    let mut found: Vec<String> = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("{}: {e}", dir.display()))
        .map(|entry| {
            entry
                .expect("an entry")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .filter_map(|name| name.strip_suffix(".json").map(str::to_owned))
        // `fling-003.coast` is a fitted start, not a transcript
        .filter(|stem| std::path::Path::new(stem).extension().is_none())
        .collect();
    found.sort();
    found
}

#[test]
fn every_transcript_in_tests_fling_is_held_to_something() {
    let listed: Vec<String> = FIXTURES
        .iter()
        .map(|(name, _)| (*name).to_owned())
        .collect();
    assert_eq!(
        transcripts(),
        listed,
        "{}: the transcripts there and FIXTURES must agree; add a new one to FIXTURES",
        dir().display()
    );
}

#[test]
fn every_coast_was_fitted_on_the_frames_held_to_the_first_allowance() {
    for (name, _) in FIXTURES {
        let transcript =
            load(&dir().join(format!("{name}.json"))).unwrap_or_else(|e| panic!("{e}"));
        #[allow(clippy::cast_precision_loss, reason = "12")]
        let held = FIRST_FRAMES as f64;
        assert!(
            (transcript.coast.fit_frames - held).abs() < f64::EPSILON,
            "{name}: fitted on {} frames, held to {FIRST_FRAMES}; refit with `python3 scripts/fling-recorder/fit.py coast crates/app/tests/fling crates/app/tests/fling/fling-*[0-9].json`",
            transcript.coast.fit_frames
        );
    }
}

/// What `name` departs from, as lines: its path, its rest frame when it
/// came to rest, the edge, going idle, and its ending as declared.
fn departures(name: &str, ending: Ending, replayed: &Replayed, max: f32) -> Vec<String> {
    let compared = &replayed.compared;
    let mut found = off_path(name, compared);
    let recorded = if came_to_rest(compared) {
        Ending::Rest
    } else {
        Ending::Cut
    };
    if recorded != ending {
        found.push(format!(
            "{name}: declared {ending:?} in FIXTURES, recorded {recorded:?}"
        ));
    }
    if ending == Ending::Rest {
        found.extend(rest_apart(name, compared));
    }
    found.extend(off_edge(name, compared, max));
    found.extend(not_idle(name, replayed));
    found
}

#[test]
fn gpu_each_recorded_coast_scrolls_as_chrome_scrolled_it() {
    let gpu = gpu();
    let (mut found, mut measured) = (Vec::new(), Vec::new());
    for &(name, ending) in FIXTURES {
        let transcript =
            load(&dir().join(format!("{name}.json"))).unwrap_or_else(|e| panic!("{e}"));
        let replayed = replay(&transcript, &gpu).unwrap_or_else(|e| panic!("{e}"));
        let max = transcript.list.scroll_height - transcript.list.height;
        found.extend(departures(name, ending, &replayed, max));
        let (first, whole) = worst(&replayed.compared);
        let gap = rest_gap(&replayed.compared);
        measured.push(format!(
            "{name}: first 12 {first:.2} px, whole {whole:.2} px over {:.0} px ({:.4}), rest gap {gap:?}",
            travelled(&replayed.compared),
            whole / travelled(&replayed.compared)
        ));
    }
    assert!(
        found.is_empty(),
        "{} departures from Chrome:\n{}\nmeasured:\n{}",
        found.len(),
        found.join("\n"),
        measured.join("\n")
    );
}
