//! The judge's floor sits over the measured noise and under one
//! instruction per row (#505).

use super::{MARGIN_PERCENT, MAX_OBSERVED_GROWTH, ONE_PER_ROW};
use crate::scaling::MIN_GROWTH_INSTRUCTIONS as FLOOR;

/// The floor read at run time, so a failed check fails the test rather
/// than the build.
fn floor() -> u64 {
    std::hint::black_box(FLOOR)
}

#[test]
fn the_floor_is_over_the_measured_noise_with_its_margin() {
    let least = MAX_OBSERVED_GROWTH * (100 + MARGIN_PERCENT) / 100;
    assert!(
        floor() >= least,
        "the floor {FLOOR} is under {least}, the noise {MAX_OBSERVED_GROWTH} \
         with {MARGIN_PERCENT}% (scaling_noise.rs): the nightly fails with nothing to blame; \
         fix: raise MIN_GROWTH_INSTRUCTIONS; reproduce: cargo test -p vapors-perf scaling_noise"
    );
}

#[test]
fn the_floor_is_under_one_instruction_per_row() {
    assert!(
        floor() < ONE_PER_ROW,
        "the floor {FLOOR} passes one instruction per row ({ONE_PER_ROW}) \
         (scaling_noise.rs); fix: lower MIN_GROWTH_INSTRUCTIONS or find the noise; reproduce: \
         cargo test -p vapors-perf scaling_noise"
    );
}
