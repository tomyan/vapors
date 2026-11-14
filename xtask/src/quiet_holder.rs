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
    ///
    /// # Errors
    ///
    /// Whether its process lives cannot be read: never taken as ended.
    pub fn ended(&self, now_ms: u64) -> Result<bool, String> {
        self.ended_given(now_ms, Owner::liveness)
    }

    /// [`Holder::ended`], with how its owner's liveness is read.
    fn ended_given(
        &self,
        now_ms: u64,
        liveness: impl Fn(&Owner) -> Result<bool, String>,
    ) -> Result<bool, String> {
        if self.left(now_ms).is_none() {
            return Ok(true);
        }
        if self.form == Form::Hold {
            return Ok(false);
        }
        liveness(&self.owner).map(|live| !live).map_err(|why| {
            format!(
                "whether the quiet lock's holder, pid {}, lives: {why}",
                self.owner.pid
            )
        })
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_holder_whose_liveness_cannot_be_read_is_refused_not_taken_as_ended() {
        // Given
        let run = Holder::me("/w/a", Duration::from_secs(60), Form::Run, "x");
        let unreadable = |_: &Owner| Err("/proc/1/stat: permission denied".to_owned());

        // When
        let ended = run.ended_given(super::super::now_ms(), unreadable);

        // Then
        let refusal = ended.expect_err("refused");
        assert!(
            refusal.contains("permission denied") && refusal.contains("lives"),
            "{refusal}"
        );
    }

    #[test]
    fn a_hold_by_hand_does_not_read_its_owner_s_liveness() {
        let hold = Holder::me("/w/a", Duration::from_secs(60), Form::Hold, "x");
        let unread = |_: &Owner| -> Result<bool, String> { panic!("read") };
        assert_eq!(hold.ended_given(super::super::now_ms(), unread), Ok(false));
    }
}
