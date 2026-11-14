//! A report per kind of frame: frames picked by kind with the warmup's
//! parity, titled by kind, with their GPU rows, and refused when a kind
//! has too few frames or a column is out of step with the frames.

use std::collections::BTreeMap;
use std::time::Duration;

use super::sections;
use crate::args::Measure;
use crate::run::Samples;

/// Six kept frames of 0 to 5 µs, the update phase the same, and GPU
/// times of 10 to 15 µs.
fn six_frames() -> Samples {
    let mut samples = Samples::default();
    for at in 0..6 {
        let time = Duration::from_micros(at);
        samples.push(time, BTreeMap::from([("update".to_owned(), time)]));
    }
    samples.gpu_time = Some((10..16).map(Duration::from_micros).collect());
    samples
}

#[test]
fn an_odd_warmup_makes_the_second_kept_frame_the_first_kinds() {
    // Given: frame i is of kind i % 2, and frame 1 is the first kept
    let mut samples = six_frames();
    samples.warmup = 1;

    // When
    let inserts = samples.of_kind(0, 2).expect("whole columns");

    // Then: kept frames 1, 3 and 5 are frames 2, 4 and 6
    let micros: Vec<_> = inserts.frame.iter().map(Duration::as_micros).collect();
    assert_eq!(micros, [1, 3, 5]);
    assert_eq!(inserts.phases["update"].len(), 3);
    let gpu: Vec<_> = inserts
        .gpu_time
        .iter()
        .flatten()
        .map(Duration::as_micros)
        .collect();
    assert_eq!(gpu, [11, 13, 15]);
}

#[test]
fn one_kind_gives_one_section_with_its_gpu_rows() {
    let reports =
        sections("10 units, Edit", &six_frames(), &["every"], Measure::Times).expect("one");
    assert_eq!(reports.len(), 1);
    assert!(
        reports[0].starts_with("### 10 units, Edit\n"),
        "{}",
        reports[0]
    );
    assert!(reports[0].contains("| GPU time |"), "{}", reports[0]);
}

#[test]
fn two_kinds_give_a_section_each_titled_by_kind_with_gpu_rows() {
    // When
    let reports = sections(
        "10 units, Insert",
        &six_frames(),
        &["insert", "remove"],
        Measure::Times,
    )
    .expect("two");

    // Then
    let titles: Vec<_> = reports
        .iter()
        .map(|r| r.lines().next().unwrap_or(""))
        .collect();
    assert_eq!(
        titles,
        [
            "### 10 units, Insert, insert frames",
            "### 10 units, Insert, remove frames"
        ]
    );
    // the insert frames are kept frames 0, 2 and 4: medians of 2 µs, and
    // 12 µs on the GPU
    assert!(reports[0].contains("| GPU time | 12.0 |"), "{}", reports[0]);
    assert!(reports[1].contains("| GPU time | 13.0 |"), "{}", reports[1]);
    assert!(
        reports[0].contains("| frame (CPU) | 2.0 |"),
        "{}",
        reports[0]
    );
}

#[test]
fn too_few_frames_of_a_kind_are_refused_naming_the_runs_to_give() {
    let mut samples = six_frames();
    samples.frame.truncate(5);
    let error = sections(
        "10 units, Insert",
        &samples,
        &["insert", "remove"],
        Measure::Times,
    )
    .expect_err("five frames");
    assert!(
        error.contains("give at least 6 runs: vapors-perf 6 10 --off-path"),
        "{error}"
    );
}

#[test]
fn a_gpu_column_out_of_step_with_the_frames_is_refused_naming_it() {
    // Given: one GPU time short
    let mut samples = six_frames();
    if let Some(times) = samples.gpu_time.as_mut() {
        times.pop();
    }

    // When
    let error = samples.of_kind(0, 2).expect_err("short");

    // Then
    assert!(
        error.starts_with("GPU time has 5 entries for 6 kept frames"),
        "{error}"
    );
}
