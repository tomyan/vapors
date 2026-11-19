//! The layout-phase clock reads the layout phase of a real lanes layout.
//! Its own test binary with one thread of tests: tracing caches whether a
//! callsite is enabled, and another thread laying out untimed at the same
//! moment can leave the span's callsite disabled for this subscriber.
//! Reproduce: cargo test --offline --test clock (in crates/perf/layout-spike).

use std::time::Duration;

use vapors_layout_spike::clock::{NoLayoutSpan, layout_phase};
use vapors_layout_spike::lanes::Lanes;
use vapors_layout_spike::{Engine, Workload};

#[test]
fn the_layout_clock_reads_more_than_zero_after_a_real_lanes_layout() {
    let mut lanes = Lanes::build(Workload::Table(30)).expect("lanes takes a table");
    let time = layout_phase(|| lanes.layout()).expect("a layout span was entered");
    assert!(time > Duration::ZERO, "{time:?}");
}

#[test]
fn the_layout_clock_refuses_work_that_enters_no_layout_span() {
    assert_eq!(layout_phase(|| {}), Err(NoLayoutSpan));
}
