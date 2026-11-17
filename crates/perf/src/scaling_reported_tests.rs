use std::collections::BTreeMap;

use super::reported_verdicts;
use crate::rows::RowChange;
use crate::scaling::{AtSize, Scaling};
use crate::scaling_judged::Judged;

/// The accessibility phase at `per_row` instructions per extra row, and
/// `style` flat.
fn accessibility(per_row: u64) -> Scaling {
    let measured: Vec<_> = [1000_u64, 3000, 10_000]
        .iter()
        .map(|&rows| AtSize {
            rows: usize::try_from(rows).expect("small"),
            phases: BTreeMap::from([
                ("accessibility".to_owned(), (rows - 1000) * per_row),
                ("style".to_owned(), 8000),
            ]),
        })
        .collect();
    Scaling::of(&measured).expect("sizes")
}

/// `PositionedInsert`'s owner and target.
fn positioned_insert() -> (super::Owner, Judged) {
    let Judged::Reported { owner, target } = Judged::of(RowChange::PositionedInsert) else {
        panic!("PositionedInsert is reported")
    };
    (owner, *target)
}

#[test]
fn every_reported_change_is_named_by_its_owners_row_in_the_plan() {
    for change in RowChange::ALL {
        // Given
        let Judged::Reported { owner, .. } = Judged::of(change) else {
            continue;
        };
        let path = format!("{}/../../{}", env!("CARGO_MANIFEST_DIR"), owner.plan);

        // When
        let plan = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{path}: {e}"));
        let row = plan
            .lines()
            .find(|line| line.starts_with(&format!("| {} |", owner.slice)));

        // Then
        let row = row.unwrap_or_else(|| panic!("{path} has no row for slice {}", owner.slice));
        assert!(
            row.contains(&format!("{change:?}")),
            "{owner}'s row does not name {change:?}"
        );
    }
}

#[test]
fn a_reported_change_still_failing_its_target_is_reported() {
    // Given: 6900 instructions per extra row, over Insert's ceiling
    let kinds = [("insert frames".to_owned(), accessibility(6900))];

    // When
    let lines = reported_verdicts(RowChange::PositionedInsert, positioned_insert(), &kinds);

    // Then
    let lines = lines.expect("reported");
    assert!(
        lines.starts_with("scaling: reported, not judged, insert frames"),
        "{lines}"
    );
    assert!(
        lines.contains("docs/plans/splice-layout.md slice 3b"),
        "{lines}"
    );
}

#[test]
fn a_reported_change_that_passes_its_target_fails_the_run() {
    // Given: every kind within Insert's ceiling
    let kinds = [
        ("insert frames".to_owned(), accessibility(1000)),
        ("remove frames".to_owned(), accessibility(1000)),
    ];

    // When
    let error = reported_verdicts(RowChange::PositionedInsert, positioned_insert(), &kinds);

    // Then
    let error = error.expect_err("now passes");
    assert!(
        error.contains("PositionedInsert now passes the rule"),
        "{error}"
    );
    assert!(error.contains("scaling_judged.rs"), "{error}");
}
