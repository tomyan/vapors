//! A declared relaxation: one phase held to its own tolerance, the
//! others to the usual one.

use std::collections::BTreeMap;

use super::{AtSize, Judged, Scaling};

/// Both phases 2.5% (25,000 instructions) above their 1000-row count.
fn both_grown_by_two_and_a_half_percent() -> Scaling {
    let measured: Vec<_> = [(1000, 1_000_000), (3000, 1_025_000), (10_000, 1_025_000)]
        .into_iter()
        .map(|(rows, count)| {
            let phases = BTreeMap::from([
                ("frame (CPU)".to_owned(), count),
                ("layout".to_owned(), count),
            ]);
            AtSize { rows, phases }
        })
        .collect();
    Scaling::of(&measured).expect("same phases at every size")
}

#[test]
fn the_relaxed_phase_passes_within_its_tolerance_and_the_others_fail_beyond_theirs() {
    // Given
    let scaling = both_grown_by_two_and_a_half_percent();
    let judged = Judged::EveryPhaseRelaxed {
        phase: "frame (CPU)",
        percent: 3,
    };

    // When
    let violations = scaling.violations(judged).expect("judged");

    // Then
    assert_eq!(violations.len(), 2, "{violations:?}");
    assert!(
        violations.iter().all(|v| v.starts_with("layout grows")),
        "{violations:?}"
    );
}

#[test]
fn the_relaxed_phase_fails_beyond_its_tolerance_naming_it() {
    // Given
    let scaling = both_grown_by_two_and_a_half_percent();
    let judged = Judged::EveryPhaseRelaxed {
        phase: "frame (CPU)",
        percent: 2,
    };

    // When
    let violations = scaling.violations(judged).expect("judged");

    // Then
    assert_eq!(violations.len(), 4, "{violations:?}");
    assert!(violations[0].contains("(over 2% and"), "{violations:?}");
}
