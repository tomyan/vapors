use std::time::Duration;

use vapors_platform::Instant;
use vapors_renderer::{GpuReading, GpuResolution, GpuTime, Samples};

use super::GpuReadout;

fn read(frame: u64, micros: u64) -> GpuTime {
    GpuTime::Read(GpuReading {
        frame,
        pairs: 1,
        time: Ok(Duration::from_micros(micros)),
        resolution: GpuResolution::Tick(Duration::from_nanos(83)),
    })
}

#[test]
fn a_line_gives_the_median_of_a_seconds_frames_with_the_sample_count() {
    // Given
    let mut readout = GpuReadout::new(Samples::Four);
    let start = Instant::now();

    // When: three frames, read in the first second, then a frame after it
    let lines: Vec<_> = [(1, 300, 0), (2, 500, 400), (3, 700, 800), (4, 900, 1000)]
        .into_iter()
        .map(|(frame, micros, at)| {
            readout.take(&read(frame, micros), start + Duration::from_millis(at))
        })
        .collect();

    // Then: the first frame says it waits, nothing more in the first
    // second, then the median of the four
    assert_eq!(lines[0].as_deref(), Some("4x waiting for GPU time"));
    assert_eq!(lines[1..3], [None, None]);
    assert_eq!(lines[3].as_deref(), Some("4x GPU 700 µs"));
}

#[test]
fn a_reading_taken_again_counts_once() {
    let mut readout = GpuReadout::new(Samples::One);
    let start = Instant::now();
    let _ = readout.take(&read(1, 300), start);
    let _ = readout.take(&read(1, 300), start);
    let line = readout.take(&read(1, 300), start + Duration::from_secs(1));
    assert_eq!(line.as_deref(), Some("1x GPU 300 µs"));
}

#[test]
fn a_device_without_timestamps_is_said_once_plainly() {
    let mut readout = GpuReadout::new(Samples::Four);
    let start = Instant::now();
    let first = readout.take(&GpuTime::Unsupported, start);
    let later = readout.take(&GpuTime::Unsupported, start + Duration::from_secs(2));
    assert_eq!(
        first.as_deref(),
        Some("4x no GPU time (no timestamp-query)")
    );
    assert_eq!(later, None);
}

#[test]
fn the_first_frame_says_the_readout_is_waiting_once() {
    let mut readout = GpuReadout::new(Samples::Four);
    let start = Instant::now();
    let first = readout.take(&GpuTime::Pending, start);
    let next = readout.take(&GpuTime::Pending, start + Duration::from_millis(16));
    assert_eq!(first.as_deref(), Some("4x waiting for GPU time"));
    assert_eq!(next, None);
}
