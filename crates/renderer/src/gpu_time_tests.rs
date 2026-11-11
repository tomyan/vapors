use std::time::Duration;

use super::{GpuMissing, GpuResolution, sum_pairs};

#[test]
fn the_gpu_time_is_the_sum_of_each_pairs_span_in_nanoseconds() {
    // Given: two passes, 10 and 30 ticks of 83.3 ns (ANV's period)
    let ticks = [100, 110, 200, 230];

    // When
    let time = sum_pairs(&ticks, 83.3);

    // Then
    assert_eq!(time, Ok(Duration::from_nanos(3332)));
}

#[test]
fn a_pass_that_ends_before_it_starts_leaves_the_frame_without_a_time() {
    // Given
    let ticks = [100, 110, 230, 200];

    // When
    let time = sum_pairs(&ticks, 1.0);

    // Then
    assert_eq!(
        time,
        Err(GpuMissing::Backwards {
            start: 230,
            end: 200
        })
    );
}

#[test]
fn a_frame_with_no_pairs_took_no_gpu_time() {
    assert_eq!(sum_pairs(&[], 1.0), Ok(Duration::ZERO));
}

#[test]
fn a_native_resolution_is_one_tick_of_the_period_truncated_to_the_nanosecond() {
    // When: ANV's period
    let resolution = GpuResolution::of_period(83.3);

    // Then
    assert_eq!(resolution, GpuResolution::Tick(Duration::from_nanos(83)));
    assert_eq!(
        resolution.to_string(),
        "83ns (the device's timestamp period)"
    );
}
