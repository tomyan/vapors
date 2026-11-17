//! The counters section: medians per phase, instructions per cycle,
//! the bytes of last-level misses, and the quiet-window note.

use std::collections::BTreeMap;

use super::{QUIET_WINDOW, counters_section};
use crate::counter_samples::CounterSamples;
use crate::run::Samples;

/// Three frames whose `update` phase ran 1000, 2000 and 3000 cycles at
/// 2.5 instructions per cycle, with 10, 20 and 30 of each miss.
fn three_frames() -> Samples {
    let mut kept = CounterSamples::default();
    for at in 1..=3 {
        let reading = [1000 * at, 2500 * at, 10 * at, 10 * at, 10 * at];
        kept.push(reading, BTreeMap::from([("update".to_owned(), reading)]));
    }
    Samples {
        counters: Some(kept),
        ..Samples::default()
    }
}

#[test]
fn a_phase_row_has_its_medians_its_ipc_and_its_llc_miss_bytes() {
    let section = counters_section("10 units, Edit", &three_frames()).expect("counted");
    assert!(
        section.contains("| update | 2000 | 2.50 | 20 | 20 | 20 | 1280 |"),
        "{section}"
    );
}

#[test]
fn a_phase_no_frame_entered_says_so_and_the_note_follows() {
    let section = counters_section("10 units, Edit", &three_frames()).expect("counted");
    assert!(section.contains("| style | no samples |"), "{section}");
    assert!(
        section.ends_with(&format!("\n{QUIET_WINDOW}\n")),
        "{section}"
    );
}

#[test]
fn samples_without_hardware_counts_are_refused_naming_the_workload() {
    let error = counters_section("10 units, Edit", &Samples::default()).expect_err("none");
    assert_eq!(error, "10 units, Edit: no hardware counts were taken");
}
