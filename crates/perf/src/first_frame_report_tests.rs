use std::collections::BTreeMap;
use std::time::Duration;

use super::report;
use crate::mount::Mounted;
use crate::sampler::FirstFrame;

fn micros(us: u64) -> Duration {
    Duration::from_micros(us)
}

fn first() -> FirstFrame {
    let start = BTreeMap::from([("style".to_owned(), micros(300))]);
    let frame = BTreeMap::from([("draw".to_owned(), micros(40))]);
    FirstFrame {
        start: micros(2000),
        start_phases: (start, Some(BTreeMap::from([("style".to_owned(), 7)]))),
        frame_cpu: micros(100),
        frame_idle: micros(150),
        frame_phases: (frame, None),
        frame_instructions: Some(9),
    }
}

const MOUNTED: Mounted = Mounted {
    time: Duration::from_micros(500),
    instructions: 11,
    rss_before_kib: 0,
    rss_after_kib: 0,
};

#[test]
fn the_summary_adds_the_mount_the_start_and_frame_0_to_idle_of_one_page() {
    // When
    let text = report(1000, MOUNTED, &first());

    // Then: 500 + 2000 + 150
    assert!(
        text.starts_with(
            "First frame, 1000 rows: mount 500.0 µs, App::start to idle 2000.0 µs, frame 0 to idle \
             150.0 µs (to submit 100.0 µs); mount to frame 0 idle 2650.0 µs.\n"
        ),
        "{text}"
    );
}

#[test]
fn each_parts_phases_are_rows_with_their_instructions_or_none() {
    // When
    let text = report(1000, MOUNTED, &first());

    // Then
    for line in [
        "| mount | all | 500.0 | 11 |",
        "| start | all | 2000.0 | none |",
        "| start | style | 300.0 | 7 |",
        "| frame | all | 100.0 | 9 |",
        "| frame | to idle | 150.0 | none |",
        "| frame | draw | 40.0 | none |",
    ] {
        assert!(
            text.lines().any(|l| l == line),
            "{line} missing from\n{text}"
        );
    }
}
