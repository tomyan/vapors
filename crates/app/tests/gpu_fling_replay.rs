//! GPU tier: each fling Chrome on Android scrolled (a transcript in
//! tests/fling/, recorded with scripts/fling-recorder on the Pixel;
//! docs/plans/touch-momentum.md), replayed through vapors, scrolls the
//! list as Chrome did: at each of Chrome's frames within `TOLERANCE`,
//! and still moving until the same frame (within `REST` of its end).

#[path = "support/fling_replay.rs"]
mod fling_replay;

use std::cmp::Ordering;
use std::path::PathBuf;

use fling_replay::{Compared, load, replay};
use vapors_gpu_harness::gpu;

/// How far vapors' offset may be from Chrome's at a frame, CSS px. To be
/// set from the first transcripts' measured drag-phase differences (the
/// touch slop vapors and Chrome apply differently).
const TOLERANCE: f32 = 2.0;

fn transcripts() -> Vec<PathBuf> {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fling");
    let mut found: Vec<PathBuf> = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("{}: {e}", dir.display()))
        .map(|entry| entry.expect("an entry").path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "json"))
        .collect();
    found.sort();
    assert!(
        !found.is_empty(),
        "{}: no fling transcripts; record them on the Pixel with scripts/fling-recorder \
         (docs/plans/touch-momentum.md) and copy them here",
        dir.display()
    );
    found
}

/// Closer to its final offset than this, a list is at rest, CSS px.
const REST: f32 = 0.5;

/// Whether vapors' offset is within `TOLERANCE` of Chrome's (not when
/// either is NaN).
fn within(frame: &Compared) -> bool {
    let off = (frame.ours - frame.chrome).abs();
    matches!(
        off.partial_cmp(&TOLERANCE),
        Some(Ordering::Less | Ordering::Equal)
    )
}

/// The last frame at which the offset `pick` reads is not yet at rest.
fn last_moving(compared: &[Compared], pick: fn(&Compared) -> f32) -> Option<usize> {
    let end = compared.last().map_or(0.0, pick);
    compared
        .iter()
        .rposition(|frame| (pick(frame) - end).abs() > REST)
}

/// Where the replay leaves Chrome's path, as lines; empty when it keeps it.
fn departures(file: &str, compared: &[Compared]) -> Vec<String> {
    let mut found: Vec<String> = compared
        .iter()
        .filter(|frame| !within(frame))
        .map(|frame| {
            format!(
                "{file} at {:.1} ms: ours {:.1}, Chrome's {:.1}",
                frame.t, frame.ours, frame.chrome
            )
        })
        .collect();
    let (ours, chrome) = (
        last_moving(compared, |f| f.ours),
        last_moving(compared, |f| f.chrome),
    );
    if ours != chrome {
        found.push(format!(
            "{file}: still moving until frame {ours:?} in vapors, {chrome:?} in Chrome"
        ));
    }
    found
}

#[test]
fn gpu_each_recorded_fling_scrolls_as_chrome_scrolled_it() {
    let gpu = gpu();
    let mut found = Vec::new();
    for path in transcripts() {
        let transcript = load(&path).unwrap_or_else(|e| panic!("{e}"));
        let compared = replay(&transcript, &gpu).unwrap_or_else(|e| panic!("{e}"));
        found.extend(departures(&transcript.file, &compared));
    }
    assert!(
        found.is_empty(),
        "{} departures from Chrome:\n{}",
        found.len(),
        found.join("\n")
    );
}
