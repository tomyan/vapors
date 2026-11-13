//! The scaling table and verdict, on counts given.

use std::collections::BTreeMap;

use super::{scaling_section, verdict};
use crate::rows::RowChange;
use crate::scaling::{AtSize, Scaling};
use crate::scaling_judged::INSERT_A11Y_PER_ROW;

fn scaling(update: [u64; 3]) -> Scaling {
    let rows = [1000, 3000, 10_000];
    let measured: Vec<_> = (0..3)
        .map(|i| AtSize {
            rows: rows[i],
            phases: BTreeMap::from([("update".to_owned(), update[i])]),
        })
        .collect();
    Scaling::of(&measured).expect("sizes")
}

#[test]
fn the_table_has_a_count_per_size_and_a_growth_per_extra_row_per_interval() {
    // Given: +2000 over 2000 rows, then +7000 over 7000
    let scaling = scaling([1000, 3000, 10_000]);

    // When
    let table = scaling_section(RowChange::Text, &scaling);

    // Then
    assert!(table.starts_with("### Scaling, Text"), "{table}");
    assert!(table.contains("| 1000 rows | 3000 rows | 10000 rows | per extra row, 1000 to 3000 | per extra row, 3000 to 10000 |"), "{table}");
    assert!(
        table.contains("| update | 1000 | 3000 | 10000 | 1.0 | 1.0 |"),
        "{table}"
    );
}

#[test]
fn growth_fails_naming_the_change_the_phase_and_the_command() {
    let error = verdict(RowChange::Text, &scaling([1000, 3000, 10_000])).expect_err("grows");
    assert!(
        error.contains("Text does work that depends on the page's size"),
        "{error}"
    );
    assert!(error.contains("update grows with the page"), "{error}");
    assert!(
        error.contains("vapors-perf 60 10 --scaling Text"),
        "{error}"
    );
}

#[test]
fn constant_counts_pass() {
    let line = verdict(RowChange::Class, &scaling([1000, 1000, 1000])).expect("flat");
    assert!(line.starts_with("scaling: pass"), "{line}");
}

/// `update` growing with the rows, the accessibility phase at
/// `per_row` instructions per extra row, and `style` flat.
fn moving(per_row: u64) -> Scaling {
    let rows = [1000, 3000, 10_000];
    let measured: Vec<_> = rows
        .iter()
        .map(|&at| AtSize {
            rows: at,
            phases: BTreeMap::from([
                ("update".to_owned(), at as u64 * 1000),
                ("accessibility".to_owned(), (at as u64 - 1000) * per_row),
                ("style".to_owned(), 8145),
            ]),
        })
        .collect();
    Scaling::of(&measured).expect("sizes")
}

#[test]
fn insert_is_judged_on_its_accessibility_phase_alone_and_the_line_says_why() {
    let line = verdict(RowChange::Insert, &moving(INSERT_A11Y_PER_ROW)).expect("at the ceiling");
    assert!(line.starts_with("scaling: pass"), "{line}");
    assert!(
        line.contains(&format!(
            "accessibility within {INSERT_A11Y_PER_ROW} instructions per extra row"
        )),
        "{line}"
    );
    assert!(line.contains("and style flat"), "{line}");
    assert!(line.contains("property 3"), "{line}");
}

#[test]
fn insert_accessibility_over_its_ceiling_fails_naming_the_phase() {
    let error = verdict(RowChange::Insert, &moving(INSERT_A11Y_PER_ROW + 1)).expect_err("over");
    assert!(error.contains("accessibility costs"), "{error}");
    assert!(
        error.contains("vapors-perf 60 10 --scaling Insert"),
        "{error}"
    );
}
