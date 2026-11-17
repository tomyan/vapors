//! The end of a run's API traffic (review of #476-#478, L6 and N3): no
//! request admitted once the session has ended, those in flight given
//! until a deadline to settle, and any left then ended and the run
//! failed, as what they cost is not known.

use std::time::{Duration, Instant};

use crate::api_budget::Budget;
use crate::api_curls::Curls;

/// How long the requests still in flight when the session ends may take
/// to settle: each one's writes to the session, now gone, fail at once or
/// within the server's 60 s write bound, and it settles then; 90 s covers
/// that. A request still waiting for its response head is not writing,
/// and is ended at this deadline.
pub const DRAIN_DEADLINE: Duration = Duration::from_secs(90);

/// How long a killed curl's request has to settle: it settles at its
/// worst case as soon as curl's output ends; 5 s covers a loaded host.
const KILLED_DEADLINE: Duration = Duration::from_secs(5);

/// No more requests admitted, and those in flight (`in_flight`) given
/// until `deadline` to settle. Any left are killed (`curls`) and settle
/// at their worst case.
///
/// # Errors
///
/// Requests were still in flight at the deadline, naming how many; or a
/// kill failing.
pub fn drained(
    budget: &Budget,
    (in_flight, curls): (&dyn Fn() -> usize, &Curls),
    deadline: Duration,
) -> Result<(), String> {
    budget.close("the session has ended");
    if waited(in_flight, deadline) {
        return Ok(());
    }
    let killed = curls.killed()?;
    let settled = waited(in_flight, KILLED_DEADLINE);
    Err(format!(
        "{killed} API requests were still in flight {deadline:?} after the session ended: \
         their curls were killed and each is charged its worst case{}",
        if settled {
            ""
        } else {
            "; some had not settled by the summary"
        }
    ))
}

/// Until `in_flight` says none, or `deadline` passes; whether none were
/// left.
fn waited(in_flight: &dyn Fn() -> usize, deadline: Duration) -> bool {
    let started = Instant::now();
    while in_flight() > 0 {
        if started.elapsed() >= deadline {
            return false;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    true
}

#[cfg(test)]
#[path = "api_build_drain_tests.rs"]
mod tests;
