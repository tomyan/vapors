//! Percentiles by nearest rank, and the worst-first lists.

use super::*;

fn sample(value: usize, label: &str) -> Sample {
    Sample {
        value,
        label: label.to_owned(),
    }
}

#[test]
fn a_percentile_is_the_value_at_the_nearest_rank() {
    // Given
    let hundred: Vec<usize> = (1..=100).collect();

    // When / Then
    assert_eq!(percentile(&[4, 1, 3, 2], 50), 2);
    assert_eq!(percentile(&hundred, 99), 99);
    assert_eq!(percentile(&hundred, 90), 90);
    assert_eq!(percentile(&[7], 1), 7);
    assert_eq!(percentile(&[], 50), 0);
}

#[test]
fn the_fitted_p99_reads_the_tail_from_the_median_and_p90() {
    // Given: main's code files on 2026-11-13, median 104 and p90 193
    let mut values = vec![104; 50];
    values.extend(vec![193; 50]);

    // When / Then: sigma 0.482, p99 104 * exp(1.121) = 319
    assert_eq!(fitted_p99(&values), 319);
    assert_eq!(fitted_p99(&[]), 0);
}

#[test]
fn the_worst_come_first_with_ties_in_label_order() {
    // Given
    let samples = [
        sample(3, "b"),
        sample(9, "z"),
        sample(3, "a"),
        sample(5, "m"),
    ];

    // When
    let lines = worst_lines(&samples, 3);

    // Then
    assert_eq!(lines, ["       9  z", "       5  m", "       3  a"]);
}

#[test]
fn a_summary_line_holds_the_count_and_percentiles() {
    // Given
    let samples: Vec<Sample> = (1..=10).map(|v| sample(v, "x")).collect();

    // When
    let lines = summary_lines("things", &samples, 1);

    // Then
    assert_eq!(lines[0], "things: n=10 median=5 p90=9 p99=10 max=10");
    assert_eq!(lines.len(), 2);
}

#[test]
fn a_band_line_counts_only_what_is_over_the_guideline_and_names_the_hard_limit() {
    // Given
    let samples = [
        sample(15, "at"),
        sample(16, "over"),
        sample(40, "far"),
        sample(50, "at the hard limit"),
        sample(80, "allowed over it"),
    ];

    // When
    let with_hard = band_lines("functions over 15 code lines", &samples, 15, Some(50), 5);
    let without = band_lines("field chains of 3+ hops", &samples, 15, None, 1);

    // Then: what is over the hard limit is not in the band
    assert_eq!(
        with_hard,
        [
            "functions over 15 code lines (hard limit 50): 3",
            "      50  at the hard limit",
            "      40  far",
            "      16  over"
        ]
    );
    assert_eq!(
        without,
        ["field chains of 3+ hops: 4", "      80  allowed over it"]
    );
}
