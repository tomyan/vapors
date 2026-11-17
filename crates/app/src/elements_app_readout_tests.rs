use std::time::Duration;

use vapors_platform::Instant;
use vapors_renderer::{GpuMissing, GpuReading, GpuResolution, GpuTime, Samples};

use super::GpuReadout;

fn reading(frame: u64, time: Result<Duration, GpuMissing>) -> GpuTime {
    GpuTime::Read(GpuReading {
        frame,
        pairs: 2,
        time,
        resolution: GpuResolution::Tick(Duration::from_nanos(83)),
    })
}

fn read(frame: u64, micros: u64) -> GpuTime {
    reading(frame, Ok(Duration::from_micros(micros)))
}

fn late(frame: u64) -> GpuTime {
    reading(frame, Err(GpuMissing::Deadline { frames: 3 }))
}

/// `readout`'s lines for `times`, each at its millisecond after a start,
/// drawn back to back.
fn lines(readout: &mut GpuReadout, times: &[(GpuTime, u64)]) -> Vec<Option<String>> {
    let times: Vec<_> = times
        .iter()
        .map(|(gpu, at)| (gpu.clone(), *at, false))
        .collect();
    lines_after(readout, &times)
}

/// As [`lines`], each frame `resumed` after a wait or not.
fn lines_after(readout: &mut GpuReadout, times: &[(GpuTime, u64, bool)]) -> Vec<Option<String>> {
    let start = Instant::now();
    times
        .iter()
        .map(|(gpu, at, resumed)| readout.take(gpu, (start + Duration::from_millis(*at), *resumed)))
        .collect()
}

#[test]
fn a_line_leads_with_the_gpu_time_as_a_share_of_the_measured_frame_interval() {
    // Given: a page drawing every 8 ms (125 Hz), its frames' GPU times
    // 1.8 and 2.0 ms in turn
    let mut readout = GpuReadout::new(Samples::Four);
    let times: Vec<_> = (0..=125_u64)
        .map(|frame| (read(frame + 1, 1800 + 200 * (frame % 2)), frame * 8))
        .collect();

    // When
    let lines = lines(&mut readout, &times);

    // Then: the share of the 8 ms interval measured, not 16.7 ms assumed
    assert_eq!(
        lines.last().cloned().flatten().as_deref(),
        Some("4x GPU 24% of 8.0 ms frame (1.9 ms mean, 2.0 ms median; 2 passes, 83 ns ticks)")
    );
}

#[test]
fn a_page_drawn_on_demand_says_it_has_no_display_interval() {
    // Given
    let mut readout = GpuReadout::new(Samples::Four);

    // When: four frames, each after a wait (`Frame::resumed`)
    let times = [
        (read(1, 300), 0, true),
        (read(2, 500), 400, true),
        (read(3, 700), 800, true),
        (read(4, 1300), 1000, true),
    ];
    let lines = lines_after(&mut readout, &times);

    // Then: the first frame says it waits, nothing more in the first
    // second, then the times, with no display interval given
    assert_eq!(
        lines[0].as_deref(),
        Some("4x waiting for GPU time (none while idle)")
    );
    assert_eq!(lines[1..3], [None, None]);
    assert_eq!(
        lines[3].as_deref(),
        Some(
            "4x GPU 0.70 ms mean, drawn on demand: no display interval (0.70 ms median; 2 passes, 83 ns ticks)"
        )
    );
}

#[test]
fn a_slow_page_drawn_back_to_back_counts_its_long_gaps() {
    // Given
    let mut readout = GpuReadout::new(Samples::Four);

    // When: four frames 200 to 400 ms apart, none after a wait
    let times = [
        (read(1, 300), 0),
        (read(2, 500), 400),
        (read(3, 700), 800),
        (read(4, 1300), 1000),
    ];
    let lines = lines(&mut readout, &times);

    // Then: the median gap, 400 ms
    assert_eq!(
        lines[3].as_deref(),
        Some("4x GPU 0% of 400.0 ms frame (0.70 ms mean, 0.70 ms median; 2 passes, 83 ns ticks)")
    );
}

#[test]
fn each_line_takes_its_own_seconds_gaps() {
    // Given: a second of frames 8 ms apart, then a second 16 ms apart
    let mut readout = GpuReadout::new(Samples::Four);
    let first = (0..=125_u64).map(|frame| (read(frame + 1, 1600), frame * 8));
    let second = (1..=63_u64).map(|frame| (read(frame + 200, 1600), 1000 + frame * 16));
    let times: Vec<_> = first.chain(second).collect();

    // When
    let lines: Vec<String> = lines(&mut readout, &times).into_iter().flatten().collect();

    // Then: the second line's interval is the second's, not a mix
    assert!(
        lines[1].starts_with("4x GPU 20% of 8.0 ms frame"),
        "{lines:?}"
    );
    assert!(
        lines[2].starts_with("4x GPU 10% of 16.0 ms frame"),
        "{lines:?}"
    );
}

#[test]
fn a_device_without_timestamps_keeps_no_gaps() {
    // Given
    let mut readout = GpuReadout::new(Samples::Four);

    // When: a long session's frames
    let times: Vec<_> = (0..1000_u64)
        .map(|frame| (GpuTime::Unsupported, frame * 8))
        .collect();
    lines(&mut readout, &times);

    // Then: nothing held for a line that never comes
    assert_eq!(readout.intervals.len(), 0);
}

#[test]
fn the_gaps_kept_between_lines_are_capped() {
    // Given: frames with no reading yet, so no line drains the gaps
    let mut readout = GpuReadout::new(Samples::Four);

    // When
    let times: Vec<_> = (0..1000_u64)
        .map(|frame| (GpuTime::Pending, frame))
        .collect();
    lines(&mut readout, &times);

    // Then
    assert_eq!(readout.intervals.len(), super::MOST_INTERVALS);
}

#[test]
fn a_reading_taken_again_counts_once() {
    // Given
    let mut readout = GpuReadout::new(Samples::One);

    // When
    let times = [
        (read(1, 300), 0, true),
        (read(1, 300), 0, true),
        (read(1, 300), 1000, true),
    ];
    let lines = lines_after(&mut readout, &times);

    // Then
    assert_eq!(
        lines[2].as_deref(),
        Some(
            "1x GPU 0.30 ms mean, drawn on demand: no display interval (0.30 ms median; 2 passes, 83 ns ticks)"
        )
    );
}

#[test]
fn a_missing_reading_is_said_with_its_reason_once_and_counted() {
    // Given
    let mut readout = GpuReadout::new(Samples::Four);

    // When: a time, two missing 100 ms apart, a time a second on
    let times = [
        (read(1, 400), 0),
        (late(2), 100),
        (late(3), 200),
        (read(4, 600), 1000),
    ];
    let lines = lines(&mut readout, &times);

    // Then: the first missing one's reason, once; the line counts both
    let why = GpuMissing::Deadline { frames: 3 }.to_string();
    assert_eq!(lines[1], Some(format!("4x GPU reading missing: {why}")));
    assert_eq!(lines[2], None);
    assert_eq!(
        lines[3].as_deref(),
        Some(
            "4x GPU 1% of 100.0 ms frame (0.50 ms mean, 0.60 ms median; 2 passes, 83 ns ticks), 2 of 4 missing"
        )
    );
}

#[test]
fn a_second_with_every_reading_missing_says_none_was_read() {
    // Given
    let mut readout = GpuReadout::new(Samples::Four);

    // When
    let lines = lines(&mut readout, &[(late(1), 0), (late(2), 1000)]);

    // Then: the reason first, which shows the readout is on, then the count
    let why = GpuMissing::Deadline { frames: 3 }.to_string();
    assert_eq!(lines[0], Some(format!("4x GPU reading missing: {why}")));
    assert_eq!(
        lines[1].as_deref(),
        Some("4x GPU none read, 2 of 2 missing")
    );
}

#[test]
fn a_device_without_timestamps_is_said_once_plainly() {
    // Given
    let mut readout = GpuReadout::new(Samples::Four);

    // When
    let times = [(GpuTime::Unsupported, 0), (GpuTime::Unsupported, 2000)];
    let lines = lines(&mut readout, &times);

    // Then
    assert_eq!(
        lines[0].as_deref(),
        Some("4x no GPU time (no timestamp-query)")
    );
    assert_eq!(lines[1], None);
}

#[test]
fn the_first_frame_says_the_readout_is_waiting_once() {
    // Given
    let mut readout = GpuReadout::new(Samples::Four);

    // When
    let lines = lines(
        &mut readout,
        &[(GpuTime::Pending, 0), (GpuTime::Pending, 16)],
    );

    // Then
    assert_eq!(
        lines[0].as_deref(),
        Some("4x waiting for GPU time (none while idle)")
    );
    assert_eq!(lines[1], None);
}

#[test]
fn one_pass_is_said_in_the_singular() {
    // When
    let shape = super::shape_of((1, GpuResolution::BrowserQuantised));

    // Then
    assert_eq!(shape, "; 1 pass, browser-rounded");
}
