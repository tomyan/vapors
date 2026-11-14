//! Percentiles and worst-first lists for the quality measurement.

/// One measured value with where it came from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sample {
    /// The value.
    pub value: usize,
    /// Where: `path:line name`.
    pub label: String,
}

/// The `p`th percentile by nearest rank: the value at position
/// `ceil(p/100 * n)` of the sorted sample, so p50 of 4 values is the
/// second and p99 of 100 the 99th. 0 for an empty sample.
pub fn percentile(values: &[usize], p: usize) -> usize {
    if values.is_empty() {
        return 0;
    }
    let mut sorted = values.to_vec();
    sorted.sort_unstable();
    let rank = (p * sorted.len()).div_ceil(100).max(1);
    sorted[rank - 1]
}

/// `n`, median, p90, p99 and max on one line, then the `worst` largest
/// samples, one per line, largest first.
pub fn summary_lines(name: &str, samples: &[Sample], worst: usize) -> Vec<String> {
    let values: Vec<usize> = samples.iter().map(|s| s.value).collect();
    let mut lines = vec![format!(
        "{name}: n={} median={} p90={} p99={} max={}",
        values.len(),
        percentile(&values, 50),
        percentile(&values, 90),
        percentile(&values, 99),
        values.iter().max().copied().unwrap_or(0)
    )];
    lines.extend(worst_lines(samples, worst));
    lines
}

/// The samples in the band, over `guideline` and within `hard` when
/// there is one (what is over the hard limit is the check's business,
/// allowed at its site or failing, and would otherwise head every list
/// on every run): `head`, the hard limit and the count on one line,
/// then the `worst` largest.
pub fn band_lines(
    head: &str,
    samples: &[Sample],
    guideline: usize,
    hard: Option<usize>,
    worst: usize,
) -> Vec<String> {
    let over: Vec<Sample> = samples
        .iter()
        .filter(|s| s.value > guideline && hard.is_none_or(|h| s.value <= h))
        .cloned()
        .collect();
    let limit = hard.map_or(String::new(), |h| format!(" (hard limit {h})"));
    let mut lines = vec![format!("{head}{limit}: {}", over.len())];
    lines.extend(worst_lines(&over, worst));
    lines
}

/// The p99 of a lognormal through the sample's median and p90: the
/// tail a cap has cut off, read from the part under it. sigma is
/// ln(p90/median) / 1.2816 (the standard normal's 90th percentile) and
/// p99 is median * exp(2.3263 * sigma). 0 for an empty sample.
#[allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)] // line counts are small integers; the fit is rounded back to one
pub fn fitted_p99(values: &[usize]) -> usize {
    let median = percentile(values, 50) as f64;
    let p90 = percentile(values, 90) as f64;
    if median <= 0.0 || p90 <= 0.0 {
        return 0;
    }
    let sigma = (p90 / median).ln() / 1.2816;
    (median * (2.3263 * sigma).exp()).round() as usize
}

/// The `count` largest samples as `    <value>  <label>`, largest first;
/// ties by label so the order is stable.
pub fn worst_lines(samples: &[Sample], count: usize) -> Vec<String> {
    let mut sorted: Vec<&Sample> = samples.iter().collect();
    sorted.sort_by(|a, b| b.value.cmp(&a.value).then_with(|| a.label.cmp(&b.label)));
    sorted
        .iter()
        .take(count)
        .map(|s| format!("    {:>4}  {}", s.value, s.label))
        .collect()
}

#[cfg(test)]
#[path = "quality_stats_tests.rs"]
mod tests;
