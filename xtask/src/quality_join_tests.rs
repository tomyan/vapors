//! The join: clippy's counts taken, unreported read as not over 1,
//! allowed sites not compared, uncompiled files skipped.

use std::collections::BTreeMap;

use super::*;
use crate::quality_measure::{Allowed, Function};

fn located(path: &str, line: usize, code_lines: usize, complexity: usize) -> Located {
    Located {
        path: path.to_owned(),
        function: Function {
            name: "f".to_owned(),
            line,
            code_lines,
            complexity,
            test: false,
            allowed: Allowed::default(),
        },
        test: false,
    }
}

fn clippy() -> ClippyCounts {
    ClippyCounts {
        lines: BTreeMap::from([(("a.rs".to_owned(), 10), 20)]),
        complexity: BTreeMap::from([(("a.rs".to_owned(), 10), 4)]),
    }
}

#[test]
fn the_join_takes_clippys_counts_treats_unreported_as_not_over_one_and_skips_uncompiled_files() {
    // Given
    let mut collected = Collected::default();
    collected.functions.push(located("a.rs", 10, 20, 4)); // reported, agrees
    collected.functions.push(located("a.rs", 30, 1, 1)); // unreported, agrees
    collected.functions.push(located("a.rs", 50, 5, 3)); // unreported, disagrees
    collected.functions.push(located("b.rs", 1, 90, 9)); // file never compiled

    // When
    let joined = join(&collected, &clippy());

    // Then
    assert_eq!(
        joined.lines_agreement,
        Agreement {
            compared: 3,
            agreed: 2
        }
    );
    assert_eq!(
        joined.complexity_agreement,
        Agreement {
            compared: 3,
            agreed: 2
        }
    );
    let clippy_lines: Vec<usize> = joined.lines.code.iter().map(|s| s.value).collect();
    assert_eq!(clippy_lines, [20, 1, 1]);
    assert_eq!(joined.mismatches.len(), 1);
    assert!(
        joined.mismatches[0]
            .label
            .starts_with("lines xtask 5 clippy 1, complexity xtask 3 clippy 1  a.rs:50 f"),
        "{}",
        joined.mismatches[0].label
    );
}

#[test]
fn a_count_allowed_at_its_site_is_not_compared() {
    // Given: 80 lines, allowed, so clippy said nothing; complexity reported
    let mut allowed = located("a.rs", 10, 80, 4);
    allowed.function.allowed.lines = true;
    let mut collected = Collected::default();
    collected.functions.push(allowed);

    // When
    let joined = join(&collected, &clippy());

    // Then
    assert_eq!(
        joined.lines_agreement,
        Agreement {
            compared: 0,
            agreed: 0
        }
    );
    assert_eq!(
        joined.complexity_agreement,
        Agreement {
            compared: 1,
            agreed: 1
        }
    );
    assert_eq!(joined.mismatches, []);
}

#[test]
fn agreement_is_in_tenths_of_a_percent_and_full_for_nothing_compared() {
    assert_eq!(
        Agreement {
            compared: 9353,
            agreed: 9349
        }
        .per_mille(),
        999
    );
    assert_eq!(Agreement::default().per_mille(), 1000);
}
