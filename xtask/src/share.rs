//! Taking a share of the gate lock (`gate_lock_shared.rs`) for a run
//! that may go beside others: `cargo xtask ci`, `cargo xtask test`, and a
//! gate's shared steps (`gate_hold.rs`). Each waits for the quiet lock
//! first, then for the machine's load if it builds, then takes its share;
//! if the quiet lock was taken meanwhile it lets the share go, so the
//! holder's drain can finish, and waits again.
//!
//! Inside a gate's step (`XTASK_GATE=1`) `ci` and `test` take nothing:
//! the gate already holds what its step needs, and a share would wait on
//! the gate's own exclusive hold. Retired if no gate step ran xtask.

use std::time::{Duration, Instant};

use crate::gate_lock_shared::{Shared, Slot, acquire_shared_as};
use crate::lock_place::Place;
use crate::quiet_lock::{self, Holder};
use crate::quiet_wait::is_mine;

/// Who takes the share, which says whether it takes a ci slot and
/// waits for the load.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sharer {
    /// `cargo xtask ci`: a slot and the load wait.
    Ci,
    /// A gate's shared steps: a slot and the load wait, as a ci run.
    Gate,
    /// `cargo xtask test`: neither, as before it took a share.
    Test,
}

impl Sharer {
    fn name(self) -> &'static str {
        match self {
            Self::Ci => "ci",
            Self::Gate => "gate",
            Self::Test => "test",
        }
    }

    fn builds(self) -> bool {
        self != Self::Test
    }
}

/// How many times a ci or test run waits for the quiet lock again when
/// it was taken between its wait and its share. RELAXED: chosen, not measured;
/// no log yet shows a hold landing in a ci run's load wait. Each round
/// is a whole quiet window (up to 60 min), so three bounds the wait at
/// about three hours, and a fourth fails naming the fix. Retired by a
/// count taken from the logs of the re-check's line.
const QUIET_ROUNDS: u32 = 3;

/// [`take`] for `sharer` at this run's place, or nothing inside a
/// gate's step.
///
/// # Errors
///
/// As [`take`], or [`Place::from_env`].
pub fn unless_in_gate(sharer: Sharer) -> Result<Option<Shared>, String> {
    let gate_env = std::env::var(crate::priority::GATE_ENV).ok();
    unless_in_gate_at(sharer, gate_env.as_deref(), Place::from_env)
}

/// [`unless_in_gate`] with the gate variable's value and the place given.
///
/// # Errors
///
/// As [`take`], or `place`.
pub fn unless_in_gate_at(
    sharer: Sharer,
    gate_env: Option<&str>,
    place: impl FnOnce() -> Result<Place, String>,
) -> Result<Option<Shared>, String> {
    if crate::priority::in_gate(false, gate_env) {
        return Ok(None);
    }
    take(&place()?, sharer).map(Some)
}

/// Wait for the quiet lock (and the load, for a sharer that builds),
/// then take a share at `place`; again if the quiet lock was taken
/// meanwhile. `ci` and `test` try at most [`QUIET_ROUNDS`] times; a
/// gate, whose lost round is another gate's exclusive steps, not a
/// failure, tries until its deadline (Tom, 2026-11-23): the quiet lock's
/// and the gate lock's longest waits together (`quiet_lock::WAIT`, from
/// gate 270's 2,885 s, and `gate_lock::WAIT`, from touche's gate times),
/// 101 min, which with one load wait is `gate_deadline::HOLD_WAIT`.
///
/// # Errors
///
/// A wait's deadline passed, naming the holder, or (`ci`, `test`) the
/// quiet lock was taken every round.
pub fn take(place: &Place, sharer: Sharer) -> Result<Shared, String> {
    if sharer == Sharer::Gate {
        return take_until(place, place.quiet_wait.saturating_add(place.lock_wait));
    }
    let who = sharer.name();
    for _ in 0..QUIET_ROUNDS {
        if let Some(share) = round(place, sharer, (place.quiet_wait, place.lock_wait))? {
            return Ok(share);
        }
    }
    Err(format!(
        "the quiet lock was taken {QUIET_ROUNDS} times between this run's wait and its share; \
         fix: rerun `cargo xtask {who}` once the quiet holders are done"
    ))
}

/// A gate's share: rounds until `deadline` from now, each wait no
/// longer than what is left of it.
fn take_until(place: &Place, deadline: Duration) -> Result<Shared, String> {
    let started = Instant::now();
    loop {
        let left = deadline.saturating_sub(started.elapsed());
        if let Some(share) = round(place, Sharer::Gate, (left, left))? {
            return Ok(share);
        }
    }
}

/// One round: the quiet wait, the load wait for a sharer that builds,
/// the share, then the re-check; none, the share let go, when the quiet
/// lock was taken meanwhile.
fn round(
    place: &Place,
    sharer: Sharer,
    (quiet_wait, lock_wait): (Duration, Duration),
) -> Result<Option<Shared>, String> {
    let who = sharer.name();
    wait_quiet(place, who, quiet_wait)?;
    let slot = if sharer.builds() {
        (place.load)(who)?;
        Slot::Take
    } else {
        Slot::None
    };
    let share = acquire_shared_as(&place.dir, (&place.worktree, who), lock_wait, slot)?;
    if !taken_since(place)? {
        return Ok(Some(share));
    }
    println!("{who}: the quiet lock was taken while this run waited; letting its share go");
    Ok(None)
}

/// Wait while someone else holds the quiet lock at `place`, up to `wait`.
fn wait_quiet(place: &Place, who: &str, wait: Duration) -> Result<(), String> {
    let mine = |holder: &Holder| is_mine(holder, place.parent, &place.worktree);
    quiet_lock::wait_clear(&place.dir, (who, &mine), wait, &mut |line| {
        println!("{line}");
    })
}

/// Whether someone else holds the quiet lock at `place` now.
fn taken_since(place: &Place) -> Result<bool, String> {
    let current = quiet_lock::current(&place.dir)?;
    Ok(current.is_some_and(|holder| !is_mine(&holder, place.parent, &place.worktree)))
}

#[cfg(test)]
#[path = "share_tests.rs"]
mod tests;
