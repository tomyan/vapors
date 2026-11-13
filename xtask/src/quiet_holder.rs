//! Who holds the quiet lock: `pid <P> in <worktree> until <unix ms>
//! running <what>`.

use std::time::Duration;

/// The holder file's one line, parsed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Holder {
    pub pid: u32,
    pub worktree: String,
    pub until_ms: u64,
    pub what: String,
}

impl Holder {
    /// This process, in `worktree`, for `held_for` from now, running `what`.
    #[must_use]
    pub fn me(worktree: &str, held_for: Duration, what: &str) -> Self {
        let held_ms = u64::try_from(held_for.as_millis()).unwrap_or(u64::MAX);
        Self {
            pid: std::process::id(),
            worktree: worktree.to_owned(),
            until_ms: super::now_ms().saturating_add(held_ms),
            what: what.to_owned(),
        }
    }

    /// The file's line.
    #[must_use]
    pub fn line(&self) -> String {
        let Self {
            pid,
            worktree,
            until_ms,
            what,
        } = self;
        format!("pid {pid} in {worktree} until {until_ms} running {what}\n")
    }

    /// `text`'s holder, or none for text of another shape.
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        let rest = text.trim().strip_prefix("pid ")?;
        let (pid, rest) = rest.split_once(" in ")?;
        let (worktree, rest) = rest.rsplit_once(" until ")?;
        let (until, what) = rest.split_once(" running ")?;
        Some(Self {
            pid: pid.parse().ok()?,
            worktree: worktree.to_owned(),
            until_ms: until.parse().ok()?,
            what: what.to_owned(),
        })
    }

    /// The time left before its deadline at `now_ms`; none once past it.
    #[must_use]
    pub fn left(&self, now_ms: u64) -> Option<Duration> {
        (self.until_ms > now_ms).then(|| Duration::from_millis(self.until_ms - now_ms))
    }

    /// `pid P in W running `what`, expires in S s`.
    #[must_use]
    pub fn describe(&self, now_ms: u64) -> String {
        let left = self.left(now_ms).unwrap_or_default();
        format!(
            "pid {} in {} running `{}`, expires in {} s",
            self.pid,
            self.worktree,
            self.what,
            left.as_secs()
        )
    }
}
