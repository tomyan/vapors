//! The two timings of an option on a workload: a first layout of a
//! freshly built tree, and a relayout after one leaf changes.

use std::time::Duration;

use crate::stats::{median, sample};
use crate::{Engine, Unsupported, Workload};

pub const FIRST_SAMPLES: usize = 9;
pub const RELAYOUT_SAMPLES: usize = 41;

/// How long a closure takes, by some clock; `Err` names why it cannot say.
pub type Clock = fn(&mut dyn FnMut()) -> Result<Duration, String>;

/// Why no time came out: the option refuses, or the clock saw nothing
/// (a broken measurement, not a gap in the table).
#[derive(Debug, PartialEq, Eq)]
pub enum Miss {
    Unsupported(Unsupported),
    Clock(String),
}

impl From<Unsupported> for Miss {
    fn from(refusal: Unsupported) -> Self {
        Self::Unsupported(refusal)
    }
}

/// The median first layout, each on a tree built outside the clock.
///
/// # Errors
/// The option cannot take the workload, or the clock saw no span.
pub fn first<E: Engine>(workload: Workload, clock: Clock) -> Result<Duration, Miss> {
    let mut built = (0..FIRST_SAMPLES)
        .map(|_| E::build(workload))
        .collect::<Result<Vec<E>, _>>()?;
    let times = sample(FIRST_SAMPLES, |i| {
        clock(&mut || built[i].layout()).map_err(Miss::Clock)
    })?;
    Ok(median(&times))
}

/// The median relayout after a flip of the leaf, change included
/// where `clock` sees it; the tree is laid out once before.
///
/// # Errors
/// The option cannot take the workload or the change, or the clock saw no span.
///
/// # Panics
/// A change taken once is refused the next time.
pub fn relayout<E: Engine>(workload: Workload, clock: Clock) -> Result<Duration, Miss> {
    let mut engine = E::build(workload)?;
    engine.layout();
    engine.change_leaf()?;
    engine.layout();
    let times = sample(RELAYOUT_SAMPLES, |_| {
        clock(&mut || {
            engine.change_leaf().expect("the first change was taken");
            engine.layout();
        })
        .map_err(Miss::Clock)
    })?;
    Ok(median(&times))
}
