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

/// `readout`'s lines for `times`, each at its millisecond after a start.
fn lines(readout: &mut GpuReadout, times: &[(GpuTime, u64)]) -> Vec<Option<String>> {
    let start = Instant::now();
    times
        .iter()
        .map(|(gpu, at)| readout.take(gpu, start + Duration::from_millis(*at)))
        .collect()
}

#[test]
fn a_line_gives_the_mean_and_median_of_a_seconds_frames_with_passes_and_step() {
    // Given
    let mut readout = GpuReadout::new(Samples::Four);

    // When: three frames read in the first second, then one after it
    let times = [
        (read(1, 300), 0),
        (read(2, 500), 400),
        (read(3, 700), 800),
        (read(4, 1300), 1000),
    ];
    let lines = lines(&mut readout, &times);

    // Then: the first frame says it waits, nothing more in the first
    // second, then the mean and median of the four
    assert_eq!(
        lines[0].as_deref(),
        Some("4x waiting for GPU time (none while idle)")
    );
    assert_eq!(lines[1..3], [None, None]);
    assert_eq!(
        lines[3].as_deref(),
        Some("4x GPU mean 700, median 700 µs; 2 passes, 83 ns ticks")
    );
}

#[test]
fn a_reading_taken_again_counts_once() {
    // Given
    let mut readout = GpuReadout::new(Samples::One);

    // When
    let times = [(read(1, 300), 0), (read(1, 300), 0), (read(1, 300), 1000)];
    let lines = lines(&mut readout, &times);

    // Then
    assert_eq!(
        lines[2].as_deref(),
        Some("1x GPU mean 300, median 300 µs; 2 passes, 83 ns ticks")
    );
}

#[test]
fn a_missing_reading_is_said_with_its_reason_once_and_counted() {
    // Given
    let mut readout = GpuReadout::new(Samples::Four);

    // When: a time, two missing, a time a second on
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
        Some("4x GPU mean 500, median 600 µs; 2 passes, 83 ns ticks, 2 of 4 missing")
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
