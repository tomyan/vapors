//! A report per kind of frame: frames picked by kind with the warmup's
//! parity, titled by kind, with their GPU rows, and refused when a kind
//! has too few frames.

use std::collections::BTreeMap;
use std::time::Duration;

use super::sections;
use crate::run::Samples;

/// Six kept frames of 0 to 5 µs, the update phase the same.
fn six_frames() -> Samples {
    let mut samples = Samples::default();
    for at in 0..6 {
        let time = Duration::from_micros(at);
        samples.push(time, BTreeMap::from([("update".to_owned(), time)]));
    }
    samples.gpu_time = Some(vec![Duration::from_micros(9); 6]);
    samples
}

#[test]
fn an_odd_warmup_makes_the_second_kept_frame_the_first_kinds() {
    // Given: frame i is of kind i % 2, and frame 1 is the first kept
    let mut samples = six_frames();
    samples.warmup = 1;

    // When
    let inserts = samples.of_kind(0, 2);

    // Then: kept frames 1, 3 and 5 are frames 2, 4 and 6
    let micros: Vec<_> = inserts.frame.iter().map(Duration::as_micros).collect();
    assert_eq!(micros, [1, 3, 5]);
    assert_eq!(inserts.phases["update"].len(), 3);
    assert_eq!(inserts.gpu_time.map(|times| times.len()), Some(3));
}

#[test]
fn one_kind_gives_one_section_with_its_gpu_rows() {
    let reports = sections("10 units, Edit", &six_frames(), &["every"], false).expect("one");
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
        false,
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
    for report in &reports {
        assert!(report.contains("| GPU time | 9.0 |"), "{report}");
    }
    // the insert frames are kept frames 0, 2 and 4: a median of 2 µs
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
    let error = sections("10 units, Insert", &samples, &["insert", "remove"], false)
        .expect_err("five frames");
    assert!(
        error.contains("give at least 6 runs: vapors-perf 6 10 --off-path"),
        "{error}"
    );
}
