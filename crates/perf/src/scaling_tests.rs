//! The judgement of growth with page size, on counts given.

use std::collections::BTreeMap;

use super::{AtSize, Judged, Scaling};
use crate::rows::RowChange;
use crate::scaling_judged::{
    FLEX_TEXT_FRAME_PERCENT, INSERT_A11Y_PER_ROW, RESIZE_A11Y_PER_ROW, SCROLL_A11Y_PER_ROW,
    SCROLL_SIZE_A11Y_PER_ROW,
};

fn size(rows: usize, phases: &[(&str, u64)]) -> AtSize {
    let phases: BTreeMap<_, _> = phases.iter().map(|&(p, n)| (p.to_owned(), n)).collect();
    AtSize { rows, phases }
}

fn scaling(counts: [u64; 3]) -> Scaling {
    let sizes = [1000, 3000, 10_000];
    let measured: Vec<_> = (0..3)
        .map(|i| size(sizes[i], &[("update", counts[i]), ("draw", 5_000_000)]))
        .collect();
    Scaling::of(&measured).expect("same phases at every size")
}

#[test]
fn counts_within_the_tolerance_of_the_first_size_pass() {
    // Given: +2% exactly at the largest size
    let scaling = scaling([1_000_000, 1_010_000, 1_020_000]);

    // When
    let violations = scaling.violations(Judged::EveryPhase).expect("judged");

    // Then
    assert!(violations.is_empty(), "{violations:?}");
}

#[test]
fn a_phase_over_the_tolerance_is_named_with_its_growth_per_row() {
    // Given: 1,000,000 to 1,090,000 over 9000 rows is 10 per row
    let scaling = scaling([1_000_000, 1_020_001, 1_090_000]);

    // When
    let violations = scaling.violations(Judged::EveryPhase).expect("judged");

    // Then: both larger sizes are over; the 10,000-row one says 10.0 a row
    assert_eq!(violations.len(), 2, "{violations:?}");
    assert!(
        violations[1].starts_with("update grows with the page"),
        "{violations:?}"
    );
    assert!(
        violations[1].contains("10.0 instructions per extra row"),
        "{violations:?}"
    );
}

#[test]
fn a_fall_is_not_growth() {
    let scaling = scaling([1_000_000, 900_000, 100_000]);
    assert_eq!(scaling.violations(Judged::EveryPhase), Ok(Vec::new()));
}

/// Update growing without bound, accessibility at 10 per extra row.
fn moving_change() -> Scaling {
    let measured = [
        size(1000, &[("update", 1), ("accessibility", 10_000)]),
        size(3000, &[("update", 1000), ("accessibility", 30_000)]),
        size(10_000, &[("update", 1_000_000), ("accessibility", 100_000)]),
    ];
    Scaling::of(&measured).expect("sizes")
}

#[test]
fn a_moving_change_is_judged_on_its_accessibility_phase_alone_against_the_ceiling() {
    // Given: 10 per extra row, at the ceiling
    let scaling = moving_change();

    // Then: the update's growth is not judged
    let at = Judged::AccessibilityPerRow { ceiling: 10 };
    assert_eq!(scaling.violations(at), Ok(Vec::new()));
}

#[test]
fn accessibility_over_its_ceiling_per_row_is_named_with_the_growth_and_the_ceiling() {
    // Given: 10 per extra row, over a ceiling of 9
    let scaling = moving_change();

    // When
    let violations = scaling
        .violations(Judged::AccessibilityPerRow { ceiling: 9 })
        .expect("judged");

    // Then: at 3000 rows and at 10,000
    assert_eq!(violations.len(), 2, "{violations:?}");
    assert!(
        violations[0].starts_with(
            "accessibility costs 10.0 instructions per extra row from 1000 to 3000 rows"
        ),
        "{}",
        violations[0]
    );
    assert!(violations[0].contains("ceiling of 9"), "{}", violations[0]);
}

#[test]
fn judging_the_accessibility_phase_without_its_count_is_refused() {
    let measured = [size(1000, &[("draw", 1)]), size(3000, &[("draw", 1)])];
    let scaling = Scaling::of(&measured).expect("sizes");
    let error = scaling
        .violations(Judged::AccessibilityPerRow { ceiling: 1 })
        .expect_err("no accessibility");
    assert!(
        error.contains("no count for phase accessibility"),
        "{error}"
    );
}

#[test]
fn a_phase_counted_at_one_size_only_is_refused_naming_it_and_the_size() {
    let missing_later = [size(1000, &[("paint", 1)]), size(3000, &[])];
    let error = Scaling::of(&missing_later).err().expect("refused");
    assert!(
        error.contains("phase paint has no count at 3000 rows"),
        "{error}"
    );
    let missing_first = [size(1000, &[]), size(3000, &[("paint", 1)])];
    let error = Scaling::of(&missing_first).err().expect("refused");
    assert!(
        error.contains("phase paint has a count at 3000 rows"),
        "{error}"
    );
}

#[test]
fn fewer_than_two_sizes_or_sizes_out_of_order_are_refused() {
    assert!(Scaling::of(&[]).is_err());
    assert!(Scaling::of(&[size(1000, &[])]).is_err());
    assert!(Scaling::of(&[size(3000, &[]), size(1000, &[])]).is_err());
}

#[test]
fn each_change_is_judged_as_the_brief_says() {
    // Idle on every phase (review-v242 L1): its counts are flat
    for change in [
        RowChange::Idle,
        RowChange::Text,
        RowChange::Class,
        RowChange::HoveredClass,
    ] {
        assert_eq!(Judged::of(change), Judged::EveryPhase);
    }
    let flex_text = Judged::EveryPhaseRelaxed {
        phase: "frame (CPU)",
        percent: FLEX_TEXT_FRAME_PERCENT,
    };
    assert_eq!(Judged::of(RowChange::FlexText), flex_text);
    let insert = Judged::AccessibilityPerRow {
        ceiling: INSERT_A11Y_PER_ROW,
    };
    assert_eq!(Judged::of(RowChange::Insert), insert);
    let resize = Judged::AccessibilityPerRow {
        ceiling: RESIZE_A11Y_PER_ROW,
    };
    assert_eq!(Judged::of(RowChange::Resize), resize);
    let scroll = Judged::AccessibilityPerRow {
        ceiling: SCROLL_A11Y_PER_ROW,
    };
    assert_eq!(Judged::of(RowChange::Scroll), scroll);
    let scroll_size = Judged::AccessibilityPerRow {
        ceiling: SCROLL_SIZE_A11Y_PER_ROW,
    };
    assert_eq!(Judged::of(RowChange::ScrollSize), scroll_size);
}

#[test]
fn growth_over_the_percentage_and_under_the_floor_passes() {
    // Given: +4.9% of 100,000 is 4,900, under the 5,000 floor
    let scaling = scaling([100_000, 104_900, 104_900]);

    // When
    let violations = scaling.violations(Judged::EveryPhase).expect("judged");

    // Then
    assert!(violations.is_empty(), "{violations:?}");
}

#[test]
fn growth_over_the_percentage_and_the_floor_fails() {
    // Given: +5,001 is over the floor and 5.001% of 100,000
    let scaling = scaling([100_000, 105_001, 105_001]);

    // When
    let violations = scaling.violations(Judged::EveryPhase).expect("judged");

    // Then
    assert_eq!(violations.len(), 2, "{violations:?}");
}

#[test]
fn growth_over_the_floor_and_under_the_percentage_passes() {
    // Given: +10,000 is over the floor and 1% of 1,000,000
    let scaling = scaling([1_000_000, 1_010_000, 1_010_000]);

    // Then
    assert_eq!(scaling.violations(Judged::EveryPhase), Ok(Vec::new()));
}
