//! The Mount tables: instructions with growth per extra row, and
//! the leaf phase that dominates at the largest size.

use super::{Measured, dominant, instructions_table};

fn measured(rows: usize, phases: &[(&str, u64)]) -> Measured {
    Measured {
        rows,
        instructions: phases.iter().map(|&(p, n)| (p.to_owned(), n)).collect(),
    }
}

fn two_sizes() -> Vec<Measured> {
    vec![
        measured(
            1000,
            &[
                ("mount", 1_000_000),
                ("style", 4_000_000),
                ("layout", 500_000),
            ],
        ),
        measured(
            3000,
            &[
                ("mount", 3_000_000),
                ("style", 12_000_000),
                ("layout", 900_000),
            ],
        ),
    ]
}

#[test]
fn the_instruction_table_gives_each_phase_per_extra_row() {
    // When
    let table = instructions_table(&two_sizes()).expect("table");

    // Then
    assert!(
        table.contains("| style | 4000000 | 12000000 | 4000.0 |"),
        "{table}"
    );
    assert!(
        table.contains("| layout | 500000 | 900000 | 200.0 |"),
        "{table}"
    );
}

#[test]
fn a_phase_missing_at_one_size_is_refused_naming_it() {
    // Given
    let mut sizes = two_sizes();
    sizes[1].instructions.remove("layout");

    // When
    let error = instructions_table(&sizes).expect_err("layout missing");

    // Then
    assert!(error.contains("layout"), "{error}");
}

#[test]
fn the_dominant_phase_is_the_largest_leaf_at_the_largest_size() {
    // When
    let line = dominant(&two_sizes()).expect("dominant");

    // Then
    assert!(
        line.starts_with("style dominates at 3000 rows: 12000000 of 15900000"),
        "{line}"
    );
}

#[test]
fn no_sizes_have_no_dominant_phase() {
    assert!(dominant(&[]).is_err());
}
