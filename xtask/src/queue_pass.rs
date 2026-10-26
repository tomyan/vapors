//! One pass of the merge queue over its open PRs, oldest first, through
//! `Ops`. Each PR is judged (`queue_judge.rs`). A skip is logged when its
//! reason changes and the pass goes on. So does a failed action, which
//! is no action taken. The first action that completes ends the pass:
//! after a merge main has moved, and a gate takes the machine's one gate
//! slot. Before acting, the PR is read again (`fresh`).

use std::time::Duration;

use crate::queue_comments::held;
use crate::queue_config::QueueConfig;
use crate::queue_decide::Decision;
use crate::queue_judge::{Judged, judge};
use crate::queue_list::Listed;
use crate::queue_log::Log;
use crate::queue_ops::Ops;
use crate::queue_state::State;

/// How long to wait before reading a PR's head again when GitHub's
/// answers disagree. Seen 2026-10-26: after a push, `gh pr view` gave the
/// old head for one or two 3 s retries, about 6 s; 10 s is over that.
pub const HEAD_RECHECK: Duration = Duration::from_secs(10);

/// What a pass works with: the effects, the settings and the log.
struct Pass<'a, O: Ops> {
    ops: &'a O,
    config: &'a QueueConfig,
    log: &'a Log,
}

/// Run one pass; with `dry_run`, judge and log only.
///
/// # Errors
///
/// The fetch of main, the PR list, or the log failing.
pub fn pass(
    ops: &impl Ops,
    config: &QueueConfig,
    log: &Log,
    state: &mut State,
    dry_run: bool,
) -> Result<(), String> {
    let pass = Pass { ops, config, log };
    ops.fetch(&["main"])?;
    let prs = ops.open_prs()?;
    for pr in &prs {
        let judged = judge(ops, config, state, pr);
        if dry_run {
            log.line(&dry(pr.number, &judged))?;
        } else if pass.act(state, pr, &prs, judged)? {
            return Ok(());
        }
    }
    if state.changed(0, "nothing to do") {
        log.line(&format!("pass over {} open PRs: nothing to do", prs.len()))?;
    }
    Ok(())
}

impl<O: Ops> Pass<'_, O> {
    /// Act on one judged PR; whether an action completed.
    fn act(
        &self,
        state: &mut State,
        pr: &Listed,
        prs: &[Listed],
        judged: Result<Judged, String>,
    ) -> Result<bool, String> {
        let judged = match judged {
            Ok(judged) => judged,
            Err(why) => {
                return self.said(
                    state,
                    pr.number,
                    &format!("#{} could not be judged: {why}", pr.number),
                );
            }
        };
        if let Decision::Skip(why) = &judged.decision {
            return self.said(state, pr.number, why);
        }
        if !self.fresh(pr, judged.decision == Decision::Merge)? {
            return Ok(false);
        }
        self.carry_out(state, pr, prs, &judged)
    }
    /// Merge or re-gate `pr` as judged; a failure is said and moves on.
    fn carry_out(
        &self,
        state: &mut State,
        pr: &Listed,
        prs: &[Listed],
        judged: &Judged,
    ) -> Result<bool, String> {
        state.changed(0, "acted");
        let done = match judged.decision {
            Decision::Merge => crate::queue_act::merge(self.ops, self.log, pr, prs),
            _ => {
                crate::queue_act::regate(self.ops, self.config, self.log, state, pr, judged.behind)
            }
        };
        let failed = done
            .err()
            .map(|why| format!("FAILED #{}: {why}", pr.number));
        failed.map_or(Ok(true), |why| self.said(state, pr.number, &why))
    }

    /// Log `reason` for PR `number` when it changed; no action was taken.
    fn said(&self, state: &mut State, number: u64, reason: &str) -> Result<bool, String> {
        if state.changed(number, reason) {
            self.log.line(reason)?;
        }
        Ok(false)
    }

    /// Whether `pr` is still as judged: its head, open, not a draft, not
    /// newly held; and, for a merge, still containing main as fetched now.
    fn fresh(&self, pr: &Listed, merging: bool) -> Result<bool, String> {
        let now = self.head_now(pr)?;
        let as_judged = now.head == pr.head
            && now.state == "OPEN"
            && !now.draft
            && !held(&self.ops.comments(pr.number)?);
        if !as_judged || !merging {
            return Ok(as_judged);
        }
        self.main_still_in(pr)
    }
    /// Whether `pr` still contains main after fetching it again.
    fn main_still_in(&self, pr: &Listed) -> Result<bool, String> {
        self.ops.fetch(&["main"])?;
        let contains = self.ops.contains_main(&pr.head)?;
        if !contains {
            self.log.line(&format!(
                "#{}: main moved since the pass began; the next pass judges it again",
                pr.number
            ))?;
        }
        Ok(contains)
    }

    /// The PR now; a head other than the judged one is read once more
    /// after `HEAD_RECHECK`, and both readings are logged.
    fn head_now(&self, pr: &Listed) -> Result<crate::queue_ops::Now, String> {
        let now = self.ops.now(pr.number)?;
        if now.head == pr.head {
            return Ok(now);
        }
        self.log.line(&format!(
            "#{} was listed at {} but reads {}; reading again in {HEAD_RECHECK:?}",
            pr.number, pr.head, now.head
        ))?;
        self.recheck_head(pr)
    }
    /// The PR read again after `HEAD_RECHECK`, logging whether it matches.
    fn recheck_head(&self, pr: &Listed) -> Result<crate::queue_ops::Now, String> {
        self.ops.wait(HEAD_RECHECK);
        let again = self.ops.now(pr.number)?;
        let said = if again.head == pr.head {
            "matches the judged head"
        } else {
            "still differs; the next pass judges it again"
        };
        self.log
            .line(&format!("#{} reads {}: {said}", pr.number, again.head))?;
        Ok(again)
    }
}

/// What a dry run logs for PR `number`.
fn dry(number: u64, judged: &Result<Judged, String>) -> String {
    match judged {
        Ok(Judged {
            decision: Decision::Merge,
            ..
        }) => format!("dry run: would merge #{number}"),
        Ok(Judged {
            decision: Decision::Regate,
            behind,
        }) => format!("dry run: would gate #{number} (behind main: {behind})"),
        Ok(Judged {
            decision: Decision::Skip(why),
            ..
        }) => format!("dry run: {why}"),
        Err(why) => format!("dry run: #{number} could not be judged: {why}"),
    }
}

#[cfg(test)]
#[path = "queue_pass_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "queue_pass_regate_tests.rs"]
mod regate_tests;
