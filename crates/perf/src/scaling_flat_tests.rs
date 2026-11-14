//! The phases a moving change holds flat beside its accessibility
//! ceiling (`Judged::AccessibilityPerRow`, `flat`).

use std::collections::BTreeMap;

use super::{AtSize, Judged, Scaling};

fn size(rows: usize, phases: &[(&str, u64)]) -> AtSize {
    let phases: BTreeMap<_, _> = phases.iter().map(|&(p, n)| (p.to_owned(), n)).collect();
    AtSize { rows, phases }
}

/// Accessibility at 10 per extra row, and style growing 45 per row, as
/// an insert's did before `docs/perf/insert-style-2026-11-13.md`.
fn moving_change_with_growing_style() -> Scaling {
    let measured = [
        size(1000, &[("accessibility", 10_000), ("style", 59_757)]),
        size(3000, &[("accessibility", 30_000), ("style", 150_876)]),
        size(10_000, &[("accessibility", 100_000), ("style", 467_510)]),
    ];
    Scaling::of(&measured).expect("sizes")
}

#[test]
fn a_phase_held_flat_beside_the_ceiling_is_named_when_it_grows() {
    // Given
    let scaling = moving_change_with_growing_style();

    // When
    let at = Judged::AccessibilityPerRow {
        ceiling: 10,
        flat: &["style"],
    };
    let violations = scaling.violations(at).expect("judged");

    // Then: style at 3000 rows and at 10,000; accessibility, at its
    // ceiling, not
    assert_eq!(violations.len(), 2, "{violations:?}");
    assert!(
        violations[0].starts_with("style grows with the page"),
        "{}",
        violations[0]
    );
}

#[test]
fn a_phase_not_held_flat_may_grow() {
    // Given
    let scaling = moving_change_with_growing_style();

    // When
    let at = Judged::AccessibilityPerRow {
        ceiling: 10,
        flat: &[],
    };

    // Then
    assert_eq!(scaling.violations(at), Ok(Vec::new()));
}
