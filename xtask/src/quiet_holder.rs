//! Who holds the quiet lock: `pid <P> in <worktree> since <unix s>
//! until <unix ms> as <form> running <what>`.

use std::time::Duration;

use crate::lock_owner::Owner;

/// How the lock was taken, which says whether its process must live.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Form {
    /// A gate's run: held while the gate's process lives.
    Gate,
    /// `quiet <minutes> -- <command>`: held while `quiet` lives.
    Run,
    /// `quiet hold`: its process exits at once; held until the deadline.
    Hold,
}

impl Form {
    fn name(self) -> &'static str {
        match self {
            Self::Gate => "gate",
            Self::Run => "run",
            Self::Hold => "hold",
        }
    }

    fn parse(text: &str) -> Option<Self> {
        [Self::Gate, Self::Run, Self::Hold]
            .into_iter()
            .find(|form| form.name() == text)
    }
}

/// The holder file's one line, parsed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Holder {
    pub owner: Owner,
    pub until_ms: u64,
    pub form: Form,
    pub what: String,
}

impl Holder {
    /// This process, in `worktree`, for `held_for` from now, as `form`,
    /// running `what`.
    #[must_use]
    pub fn me(worktree: &str, held_for: Duration, form: Form, what: &str) -> Self {
        let held_ms = u64::try_from(held_for.as_millis()).unwrap_or(u64::MAX);
        let owner = Owner::parse(&crate::lock_owner::line(worktree)).expect("our own line");
        Self {
            owner,
            until_ms: super::now_ms().saturating_add(held_ms),
            form,
            what: what.to_owned(),
        }
    }

    /// The file's line.
    #[must_use]
    pub fn line(&self) -> String {
        let Owner {
            pid,
            worktree,
            since,
        } = &self.owner;
        let (until, form, what) = (self.until_ms, self.form.name(), &self.what);
        format!("pid {pid} in {worktree} since {since} until {until} as {form} running {what}\n")
    }

    /// `text`'s holder, or none for text of another shape.
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        let (owner, rest) = text.trim().split_once(" until ")?;
        let (until, rest) = rest.split_once(" as ")?;
        let (form, what) = rest.split_once(" running ")?;
        Some(Self {
            owner: Owner::parse(owner)?,
            until_ms: until.parse().ok()?,
            form: Form::parse(form)?,
            what: what.to_owned(),
        })
    }

    /// The time left before its deadline at `now_ms`; none once past it.
    #[must_use]
    pub fn left(&self, now_ms: u64) -> Option<Duration> {
        (self.until_ms > now_ms).then(|| Duration::from_millis(self.until_ms - now_ms))
    }

    /// Whether it no longer holds the lock at `now_ms`: past its
    /// deadline, or, but for a hold, its process gone (or its pid reused).
    #[must_use]
    pub fn ended(&self, now_ms: u64) -> bool {
        self.left(now_ms).is_none() || (self.form != Form::Hold && !self.owner.live())
    }

    /// `pid P in W running `what` (form), expires in S s`.
    #[must_use]
    pub fn describe(&self, now_ms: u64) -> String {
        let left = self.left(now_ms).unwrap_or_default();
        format!(
            "pid {} in {} running `{}` ({}), expires in {} s",
            self.owner.pid,
            self.owner.worktree,
            self.what,
            self.form.name(),
            left.as_secs()
        )
    }
}
