//! The signals section: its shape, its length and what it fails on.

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
    for n in 0..20 {
        collected
            .function_lines
            .code
            .push(sample(16 + n, &format!("a.rs:{n} f{n}")));
        collected
            .complexity
            .code
            .push(sample(7, &format!("a.rs:{n} f{n}")));
    }
    collected.chains.push(sample(2, "a.rs:1 s.a.b"));
    collected.chains.push(sample(4, "a.rs:2 s.a.b.c.d"));
    collected
        .function_lines
        .code
        .push(sample(124, "a.rs:90 allowed_over_the_hard_limit"));
    collected
}

fn thresholds() -> (Guideline, Hard) {
    (
        Guideline {
            function_lines: 15,
            complexity: 6,
            file_lines: 220,
            chain_hops: 3,
        },
        Hard {
            function_lines: 50,
            complexity: 12,
            file_lines: 320,
        },
    )
}

#[test]
fn the_signals_fit_in_fifteen_lines_with_counts_and_the_worst() {
    // Given
    let (guideline, hard) = thresholds();

    // When
    let lines = signal_lines(&collected(), &guideline, &hard);

    // Then
    assert!(lines.len() <= 15, "{} lines", lines.len());
    assert!(
        lines[0].starts_with("quality signals (advisory"),
        "{}",
        lines[0]
    );
    assert_eq!(lines[1], "functions over 15 code lines (hard limit 50): 20");
    assert_eq!(lines[2], "      35  a.rs:19 f19");
    assert_eq!(lines[5], "complexity over 6 (hard limit 12): 20");
    assert!(lines.contains(&"complexity over 6 (hard limit 12): 20".to_owned()));
    assert!(lines.contains(&"files over 220 lines (hard limit 320): 1".to_owned()));
    assert!(lines.contains(&"field chains of 3+ hops: 1".to_owned()));
    assert!(lines.contains(&"       4  a.rs:2 s.a.b.c.d".to_owned()));
}

#[test]
fn the_list_is_one_sample_per_line_with_its_metric() {
    let lines = list_lines(&collected());
    assert_eq!(lines[0], "file_lines\t230\ta.rs");
    assert_eq!(lines.len(), 2 + 21 + 20 + 2);
}

#[test]
fn the_mode_follows_the_flags() {
    assert_eq!(Mode::from_flags(false, false), Mode::Signals);
    assert_eq!(Mode::from_flags(true, false), Mode::Distributions);
    assert_eq!(Mode::from_flags(false, true), Mode::List);
}
