//! GPU tier: each fling coast Chrome on Android scrolled (a transcript
//! in tests/fling/, recorded with scripts/fling-recorder on the Pixel,
//! and its fitted start beside it; docs/plans/touch-momentum.md),
//! started in vapors where and as fast as Chrome's began, scrolls the
//! list as Chrome did: within `FIRST_ALLOWANCE` over the first
//! `FIRST_FRAMES` frames, within `WHOLE_ALLOWANCE` over the coast, and
//! coming to rest (within `REST` of its end) within `REST_FRAMES_APART`
//! of Chrome's frame, at an edge where Chrome's stopped at one.

#[path = "support/fling_replay.rs"]
mod fling_replay;

use std::path::PathBuf;

use fling_replay::{Compared, load, replay};
use vapors_gpu_harness::gpu;

/// The coast's first frames, held to `FIRST_ALLOWANCE`.
const FIRST_FRAMES: usize = 12;
/// How far vapors' offset may be from Chrome's over the first frames,
/// CSS px, and over the whole coast. These are recording noise, not
/// slack for vapors: Chrome's own curve, fitted to Chrome's recordings,
/// is off by up to 5.6 px over the first 12 frames and 10.1 px over a
/// coast (010; the plan's Pixel findings), rounded up. Its frames are
/// quantised to device pixels and its start time is fitted.
const FIRST_ALLOWANCE: f32 = 6.0;
const WHOLE_ALLOWANCE: f32 = 11.0;
/// Closer to its final offset than this, a list is at rest, CSS px.
const REST: f32 = 0.5;
/// How many frames apart vapors and Chrome may come to rest. Chrome's
/// last moving frame moves 0.4 to 0.7 px, past where its fitted curve
/// ends, so the curve comes to rest a frame early on 006, 021 and 023.
/// Retires with #395 (Chrome's fling resampling), if that
/// explains the tail.
const REST_FRAMES_APART: usize = 1;

const FIXTURES: &[&str] = &[
    "fling-003",
    "fling-005",
    "fling-006",
    "fling-010",
    "fling-016",
    "fling-019",
    "fling-021",
    "fling-022",
    "fling-023",
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

/// Whether `off` is at most `allowance` (not when it is NaN).
fn within(off: f32, allowance: f32) -> bool {
    off.abs() <= allowance
}

/// The last frame at which the offset `pick` reads is not yet at rest.
fn last_moving(compared: &[Compared], pick: fn(&Compared) -> f32) -> Option<usize> {
    let end = compared.last().map_or(0.0, pick);
    compared
        .iter()
        .rposition(|frame| (pick(frame) - end).abs() > REST)
}

/// Frames off Chrome's path by more than their allowance, as lines.
fn off_path(file: &str, compared: &[Compared]) -> Vec<String> {
    compared
        .iter()
        .enumerate()
        .filter(|(at, frame)| {
            let allowance = if *at < FIRST_FRAMES {
                FIRST_ALLOWANCE
            } else {
                WHOLE_ALLOWANCE
            };
            !within(frame.ours - frame.chrome, allowance)
        })
        .map(|(at, frame)| {
            format!(
                "{file} frame {at} at {:.1} ms: ours {:.1}, Chrome's {:.1}",
                frame.t, frame.ours, frame.chrome
            )
        })
        .collect()
}

/// Where the replay stops apart from Chrome, as lines.
fn rest_apart(file: &str, compared: &[Compared]) -> Vec<String> {
    let mut found = Vec::new();
    let (ours, chrome) = (
        last_moving(compared, |f| f.ours),
        last_moving(compared, |f| f.chrome),
    );
    let apart = ours.zip(chrome).map(|(ours, chrome)| ours.abs_diff(chrome));
    if apart.is_none_or(|apart| apart > REST_FRAMES_APART) {
        found.push(format!(
            "{file}: still moving until frame {ours:?} in vapors, {chrome:?} in Chrome"
        ));
    }
    if let Some(last) = compared
        .last()
        .filter(|f| !within(f.ours - f.chrome, WHOLE_ALLOWANCE))
    {
        found.push(format!(
            "{file}: at rest at {:.1}, Chrome at {:.1}",
            last.ours, last.chrome
        ));
    }
    found
}

/// Chrome's coast ending this close to an end of the list ended there,
/// CSS px: its `scrollTop` is quantised to device pixels (1/2.75 px).
const AT_EDGE: f32 = 1.0;

/// Where Chrome's coast stopped at an end of the list and vapors' did
/// not stop exactly there, as a line.
fn off_edge(file: &str, compared: &[Compared], max: f32) -> Option<String> {
    let last = compared.last()?;
    let edge = [0.0, max]
        .into_iter()
        .find(|edge| (last.chrome - edge).abs() <= AT_EDGE)?;
    #[allow(
        clippy::float_cmp,
        reason = "a scroll clamped to its end is exactly there"
    )]
    let at_edge = last.ours == edge;
    (!at_edge).then(|| {
        format!(
            "{file}: Chrome stopped at the edge {edge}, vapors at {:.2}",
            last.ours
        )
    })
}

#[test]
fn every_transcript_in_tests_fling_is_held_to_something() {
    let listed: Vec<String> = FIXTURES.iter().map(|name| (*name).to_owned()).collect();
    assert_eq!(
        transcripts(),
        listed,
        "{}: the transcripts there and FIXTURES must agree; add a new one to FIXTURES",
        dir().display()
    );
}

#[test]
fn gpu_each_recorded_coast_scrolls_as_chrome_scrolled_it() {
    let gpu = gpu();
    let mut found = Vec::new();
    for name in FIXTURES {
        let transcript =
            load(&dir().join(format!("{name}.json"))).unwrap_or_else(|e| panic!("{e}"));
        let compared = replay(&transcript, &gpu).unwrap_or_else(|e| panic!("{e}"));
        found.extend(off_path(name, &compared));
        found.extend(rest_apart(name, &compared));
        let max = transcript.list.scroll_height - transcript.list.height;
        found.extend(off_edge(name, &compared, max));
    }
    assert!(
        found.is_empty(),
        "{} departures from Chrome:\n{}",
        found.len(),
        found.join("\n")
    );
}
