//! A phase's times over the samples: median, 90th percentile and
//! maximum, by nearest rank, never a mean.

use std::time::Duration;

/// Median, 90th percentile and maximum of a set of samples.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Distribution {
    /// The sample at rank ⌈n/2⌉ of the sorted samples.
    pub median: Duration,
    /// The sample at rank ⌈0.9 n⌉.
    pub p90: Duration,
    /// The largest sample.
    pub max: Duration,
}

impl Distribution {
    /// The distribution of `samples`; `None` when there are none.
    #[must_use]
    pub fn of(samples: &[Duration]) -> Option<Self> {
        let mut sorted = samples.to_vec();
        sorted.sort_unstable();
        Some(Self {
            median: *sorted.get(rank(sorted.len(), 50)?)?,
            p90: sorted[rank(sorted.len(), 90)?],
            max: *sorted.last()?,
        })
    }
}

/// The zero-based index of the nearest-rank `percent` of `count` samples.
fn rank(count: usize, percent: usize) -> Option<usize> {
    (count * percent).div_ceil(100).checked_sub(1)
}

#[cfg(test)]
mod tests {
    use super::Distribution;
    use std::time::Duration;

    fn micros(values: &[u64]) -> Vec<Duration> {
        values.iter().copied().map(Duration::from_micros).collect()
    }

    #[test]
    fn ten_samples_give_the_fifth_the_ninth_and_the_largest() {
        // Given
        let samples = micros(&[10, 1, 9, 2, 8, 3, 7, 4, 6, 5]);

        // When
        let distribution = Distribution::of(&samples).expect("samples");

        // Then
        assert_eq!(
            distribution,
            Distribution {
                median: Duration::from_micros(5),
                p90: Duration::from_micros(9),
                max: Duration::from_micros(10),
            }
        );
    }

    #[test]
    fn one_sample_is_every_statistic() {
        let one = Distribution::of(&micros(&[7])).expect("a sample");
        assert_eq!(
            [one.median, one.p90, one.max],
            [Duration::from_micros(7); 3]
        );
    }

    #[test]
    fn no_samples_have_no_distribution() {
        assert_eq!(Distribution::of(&[]), None);
    }
}
