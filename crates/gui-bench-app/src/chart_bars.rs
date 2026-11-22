//! The chart's bar heights as `style:height` reads them: the spec's
//! `bar_height` and `previous_height` for a series (`0..SEGMENTS.len()`)
//! and a slot (`0..CHART_BARS`); a series or slot past them is refused.
use crate::dashboard::{CHART_BARS, SEGMENTS, bar_height, previous_height as spec_previous};

fn check(series: usize, slot: usize) {
    assert!(
        series < SEGMENTS.len() && slot < CHART_BARS,
        "series {series} slot {slot} is outside {} series of {CHART_BARS} bars (cargo test -p vapors-gui-bench-app chart_heights)",
        SEGMENTS.len()
    );
}

/// The height in px of the current period's bar.
pub fn current_height(series: usize, slot: usize) -> f32 {
    check(series, slot);
    bar_height(series, slot)
}

/// The height in px of the previous period's bar.
pub fn previous_height(series: usize, slot: usize) -> f32 {
    check(series, slot);
    spec_previous(series, slot)
}
