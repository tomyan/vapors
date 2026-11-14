//! The measurement report: its tables, the fit, the agreement, the refusal.

use super::*;
use crate::quality_stats::Sample;

fn sample(value: usize, label: &str) -> Sample {
    Sample {
        value,
        label: label.to_owned(),
    }
}

fn collected() -> Collected {
    let mut collected = Collected::default();
    collected.files.code.push(sample(230, "a.rs"));
    collected.files.tests.push(sample(100, "a_tests.rs"));
    collected.chains.push(sample(4, "a.rs:2 s.a.b.c.d"));
    collected
}

#[test]
fn the_report_prints_clippys_counts_the_fit_and_the_agreement() {
    // Given
    let mut joined = Joined {
        lines_agreement: Agreement {
            compared: 1000,
            agreed: 999,
        },
        complexity_agreement: Agreement {
            compared: 1000,
            agreed: 980,
        },
        ..Joined::default()
    };
    joined.lines.code.push(sample(20, "a.rs:1 f"));

    // When
    let report = distribution_report(&collected(), &joined);

    // Then
    let heads: Vec<&String> = report
        .lines
        .iter()
        .filter(|l| !l.starts_with(' '))
        .collect();
    assert!(
        heads[0].starts_with("file lines, code: n=1 median=230"),
        "{}",
        heads[0]
    );
    assert_eq!(
        heads[2],
        "file lines, code: lognormal through median and p90 gives p99 230"
    );
    assert!(
        heads[4]
            .starts_with("function code lines (clippy's too_many_lines, threshold 1), code: n=1"),
        "{}",
        heads[4]
    );
    assert!(
        heads
            .contains(&&"agreement with clippy on lines: 999/1000 (99.9%, floor 99.9%)".to_owned()),
        "{heads:?}"
    );
    assert!(heads.contains(
        &&"agreement with clippy on complexity: 980/1000 (98.0%, floor 97.0%)".to_owned()
    ));
    assert!(report.refusal.is_none());
}

#[test]
fn agreement_under_a_floor_is_a_refusal_after_the_tables() {
    // Given
    let joined = Joined {
        complexity_agreement: Agreement {
            compared: 1000,
            agreed: 969,
        },
        ..Joined::default()
    };

    // When
    let report = distribution_report(&collected(), &joined);

    // Then
    let refusal = report.refusal.expect("refused");
    assert!(
        refusal.starts_with("xtask's counts have drifted from clippy's"),
        "{refusal}"
    );
    assert!(!report.lines.is_empty(), "the tables are still printed");
}
