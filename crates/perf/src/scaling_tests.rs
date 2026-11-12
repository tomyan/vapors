//! The judgement of growth with page size, on counts given.

use std::collections::BTreeMap;

use super::{AtSize, Judged, Scaling};
use crate::rows::RowChange;

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

#[test]
fn an_idle_update_is_judged_on_the_update_total_only() {
    // Given: draw grows, update does not
    let measured = [
        size(1000, &[("update", 100_000), ("draw", 100_000)]),
        size(3000, &[("update", 100_000), ("draw", 900_000)]),
    ];
    let scaling = Scaling::of(&measured).expect("sizes");

    // Then
    assert_eq!(scaling.violations(Judged::UpdateTotal), Ok(Vec::new()));
    assert_eq!(
        scaling
            .violations(Judged::EveryPhase)
            .expect("judged")
            .len(),
        1
    );
}

#[test]
fn nothing_is_judged_for_insert_which_is_reported_only() {
    let scaling = scaling([1, 1000, 1_000_000]);
    assert_eq!(Judged::of(RowChange::Insert), Judged::Nothing);
    assert_eq!(scaling.violations(Judged::Nothing), Ok(Vec::new()));
}

#[test]
fn judging_the_update_total_without_an_update_count_is_refused() {
    let measured = [size(1000, &[("draw", 1)]), size(3000, &[("draw", 1)])];
    let scaling = Scaling::of(&measured).expect("sizes");
    let error = scaling
        .violations(Judged::UpdateTotal)
        .expect_err("no update");
    assert!(error.contains("no count for phase update"), "{error}");
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
    assert_eq!(Judged::of(RowChange::Idle), Judged::UpdateTotal);
    for change in [RowChange::Text, RowChange::Class, RowChange::HoveredClass] {
        assert_eq!(Judged::of(change), Judged::EveryPhase);
    }
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
