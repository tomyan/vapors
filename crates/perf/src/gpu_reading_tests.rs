//! A frame's GPU time is taken only from its own reading.

use std::time::Duration;

use vapors_app::{GpuMissing, GpuReading, GpuTime};

use super::frame_gpu_time;

fn read(frame: u64, time: Result<Duration, GpuMissing>) -> GpuTime {
    GpuTime::Read(GpuReading {
        frame,
        pairs: 2,
        time,
    })
}

#[test]
fn the_frames_own_reading_gives_its_time() {
    let time = Duration::from_micros(250);
    assert_eq!(frame_gpu_time(read(4, Ok(time)), 4), Ok(Some(time)));
}

#[test]
fn a_device_without_timestamp_queries_gives_no_time_and_no_error() {
    assert_eq!(frame_gpu_time(GpuTime::Unsupported, 4), Ok(None));
}

#[test]
fn no_reading_another_frames_reading_or_a_missing_time_are_refused() {
    let cases = [
        (GpuTime::Pending, "no GPU time was read back"),
        (read(3, Ok(Duration::ZERO)), "is frame 3's"),
        (
            read(4, Err(GpuMissing::NotSubmitted)),
            "dropped unsubmitted",
        ),
    ];
    for (time, says) in cases {
        let refusal = frame_gpu_time(time, 4).expect_err("refused");
        assert!(refusal.contains(says), "{refusal}");
    }
}
