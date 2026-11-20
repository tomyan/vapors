//! Samples to numbers: the median and the minimum of a few runs.

use std::time::{Duration, Instant};

/// The sorted times of `samples` runs of `run`, each given its index.
///
/// # Errors
/// The first error of a run.
pub fn sample<E>(
    samples: usize,
    mut run: impl FnMut(usize) -> Result<Duration, E>,
) -> Result<Vec<Duration>, E> {
    let mut times = (0..samples).map(&mut run).collect::<Result<Vec<_>, E>>()?;
    times.sort();
    Ok(times)
}

/// The median of sorted `times`.
///
/// # Panics
/// `times` is empty: a harness that took no samples has nothing to say.
#[must_use]
pub fn median(times: &[Duration]) -> Duration {
    assert!(
        !times.is_empty(),
        "no samples; run `cargo run --profile timing --features all --bin bench`"
    );
    times[times.len() / 2]
}

/// How long `work` takes.
pub fn timed(work: impl FnOnce()) -> Duration {
    let start = Instant::now();
    work();
    start.elapsed()
}
