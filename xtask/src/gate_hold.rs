//! The hold a gate keeps on the machine while one step runs: a share of
//! the gate lock for a correctness step, beside other gates' and ci runs'
//! shares; or, for a step whose result load can break, the quiet lock and
//! the gate lock exclusive, so it runs alone. Which step takes which is
//! `[gate.shared]` and `[gate.exclusive]` (`gate_hold_table.rs`).
//!
//! The hold is kept while consecutive steps need the same one. When it
//! changes, the old hold is released before the new one is asked for:
//! flock counts this process's share and its exclusive lock as two
//! owners, so asking for the exclusive lock while holding the share would
//! wait on itself. Going to exclusive, the order is release, then the
//! quiet lock, then the turn and the exclusive lock: a gate that took the
//! quiet lock while still holding its share could wait for another gate's
//! share while that gate waits for its quiet lock.

use crate::gate_lock;
use crate::gate_lock_shared::Shared;
use crate::gate_plan::PlannedStep;
use crate::lock_place::Place;
use crate::quiet_lock::{self, Form, Holder, LONGEST, Quiet};
use crate::quiet_wait::gate_takes_a_hold;
use crate::share::{self, Sharer};

/// The hold a step needs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Hold {
    /// Beside other shares: a correctness check.
    Shared,
    /// Alone: a check that load can break.
    Exclusive,
}

/// What a gate holds now.
#[derive(Debug)]
enum Held {
    Nothing,
    Shared {
        _share: Shared,
    },
    /// The quiet lock (none when its holder is the gate's own) and the
    /// gate lock exclusive.
    Exclusive {
        _quiet: Option<Quiet>,
        _lock: gate_lock::Held,
    },
}

impl Held {
    fn is(&self, hold: Hold) -> bool {
        matches!(
            (self, hold),
            (Self::Shared { .. }, Hold::Shared) | (Self::Exclusive { .. }, Hold::Exclusive)
        )
    }
}

/// A gate's holds over its run.
#[derive(Debug)]
pub struct GateHolds {
    place: Place,
    held: Held,
}

impl GateHolds {
    /// Nothing held yet, at `place`.
    #[must_use]
    pub fn new(place: Place) -> Self {
        Self {
            place,
            held: Held::Nothing,
        }
    }

    /// Hold `hold` for the next step: kept if held already, else the old
    /// hold released and `hold` taken.
    ///
    /// # Errors
    ///
    /// A wait's deadline passed, naming the holder; nothing is held then.
    pub fn hold(&mut self, hold: Hold) -> Result<(), String> {
        if self.held.is(hold) {
            return Ok(());
        }
        self.held = Held::Nothing;
        self.held = match hold {
            Hold::Shared => Held::Shared {
                _share: share::take(&self.place, Sharer::Gate)?,
            },
            Hold::Exclusive => exclusive(&self.place)?,
        };
        Ok(())
    }
}

/// Each step's hold, from the tables, and the holds a gate keeps.
#[derive(Debug)]
pub struct StepHolds {
    table: std::collections::BTreeMap<String, Hold>,
    holds: GateHolds,
}

impl StepHolds {
    /// Every step of `planned` classified by `config`'s tables, at this
    /// run's place; nothing held yet.
    ///
    /// # Errors
    ///
    /// As [`crate::gate_hold_table::holds`], or [`Place::from_env`].
    pub fn new(config: &crate::config::Config, planned: &[PlannedStep]) -> Result<Self, String> {
        Ok(Self {
            table: crate::gate_hold_table::holds(config, planned)?,
            holds: GateHolds::new(Place::from_env()?),
        })
    }

    /// Hold what `step` needs.
    ///
    /// # Errors
    ///
    /// As [`GateHolds::hold`], or `step` is not in the table it was
    /// classified from.
    pub fn before(&mut self, step: &PlannedStep) -> Result<(), String> {
        let hold = self.table.get(&step.name).copied().ok_or_else(|| {
            format!(
                "`{}` was not classified when the gate began; fix: report it, this is a bug in xtask",
                step.name
            )
        })?;
        self.holds.hold(hold)
    }
}

/// The quiet lock, unless its holder is the gate's own, then the gate
/// lock exclusive, then the load wait: what a gate took for its whole run
/// before its holds were per step.
fn exclusive(place: &Place) -> Result<Held, String> {
    let current = quiet_lock::current(&place.dir)?;
    let quiet = if gate_takes_a_hold(current.as_ref(), place.parent, &place.worktree) {
        let holder = Holder::me(&place.worktree, LONGEST, Form::Gate, "gate");
        Some(quiet_lock::take(&place.dir, holder, place.quiet_wait)?)
    } else {
        None
    };
    let held = gate_lock::acquire(&place.dir, &place.worktree, place.lock_wait)?;
    (place.load)("gate")?;
    Ok(Held::Exclusive {
        _quiet: quiet,
        _lock: held,
    })
}

#[cfg(test)]
#[path = "gate_hold_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "gate_hold_process_tests.rs"]
mod process_tests;
