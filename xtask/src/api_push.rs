//! `request_push`, outside (`docs/plans/api-builder.md`, the lead's answer
//! 8): the builder's branch fetched into a bare repository of this
//! call's own (`api_push_fetch.rs`), its history and diff checked there
//! (`api_push_history.rs`, `api_push_checks.rs`), `cargo xtask ci` run on
//! it in a fresh sandbox (`api_push_ci.rs`), then the checked commit
//! pushed and a PR opened (`api_push_remote.rs`). No git runs in the
//! builder's clone outside the sandbox. One push runs at a time, and the
//! commit pushed is the one checked (review of slice 4, H1).

use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use crate::api_clone::ORIGIN_MAIN;
use crate::api_paths::Builder;
use crate::api_push_fetch::git as bare_git;
use crate::api_sandbox::Host;

/// How long a `request_push` call may take, all of it: the sandboxed ci
/// (`api_push_ci::CI_DEADLINE`) and 15 minutes for the rest, whose own
/// deadlines are each a minute or two. The push itself starts only if
/// it can end within this (`time_left_to_push`), so the session is never
/// told a call failed that then pushed (review of slice 4, M3).
pub const PUSH_CALL_DEADLINE: Duration =
    Duration::from_secs(crate::api_push_ci::CI_DEADLINE.as_secs() + 15 * 60);

/// What a push needs, and the lock and count of its calls.
pub struct Pusher {
    pub main: PathBuf,
    pub builder: Builder,
    pub host: Host,
    /// The paths the branch may change (`api_push_checks::allowed`).
    pub allowed: Vec<String>,
    /// The calls so far; held for the whole of a call.
    calls: Mutex<u64>,
}

/// Refuses a session builder's name that a ci builder could take
/// (`Builder::ci`, review of slice 4, L6).
///
/// # Errors
///
/// A name holding `-ci-`, naming it.
pub fn session_name(name: &str) -> Result<(), String> {
    if name.contains("-ci-") {
        return Err(format!(
            "builder name {name:?}: `-ci-` names request_push's ci builders; choose another"
        ));
    }
    Ok(())
}

impl Pusher {
    /// The pusher for `builder`'s branch.
    pub fn new(main: PathBuf, builder: Builder, host: Host, allowed: Vec<String>) -> Self {
        Self {
            main,
            builder,
            host,
            allowed,
            calls: Mutex::new(0),
        }
    }

    /// The branch checked, built, pushed, and its PR opened with `title`
    /// and `body`; what was done and the PR's URL. One call at a time.
    ///
    /// # Errors
    ///
    /// Any step refusing, its reason and output: the text the session reads.
    pub fn push(&self, title: &str, body: &str) -> Result<String, String> {
        let started = Instant::now();
        let mut calls = self
            .calls
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        *calls += 1;
        crate::api_push_remote::pr_text(title, body)?;
        let branch = crate::api_fences_run::branch(&self.builder);
        let objects = &self.host.objects;
        let bare = crate::api_push_fetch::fetched(&self.builder, objects, (&branch, *calls))?;
        let pushed = self.checked_built_pushed(&bare, (&branch, *calls), started, (title, body));
        let _ = std::fs::remove_dir_all(&bare); // this call's; the outcome is the result
        pushed
    }

    fn checked_built_pushed(
        &self,
        bare: &Path,
        (branch, call): (&str, u64),
        started: Instant,
        (title, body): (&str, &str),
    ) -> Result<String, String> {
        let tip = bare_git(bare, &["rev-parse", &format!("refs/heads/{branch}")])?;
        let lines = self.checked(bare, &tip)?;
        crate::api_push_ci::built(self, (bare, &tip), call)?;
        time_left_to_push(started.elapsed())?;
        crate::api_push_remote::pushed(&self.main, bare, (&tip, branch))?;
        let url = crate::api_push_remote::opened(&self.main, branch, (title, body))?;
        Ok(format!(
            "pushed {tip} to {branch} ({lines} lines) and opened {url}"
        ))
    }

    /// The commits from where `tip` left `origin/main` checked to be a
    /// line, and their diff checked; the lines it changes.
    fn checked(&self, bare: &Path, tip: &str) -> Result<u64, String> {
        let origin_main = crate::queue_git::git(&self.main, &["rev-parse", ORIGIN_MAIN])?;
        let fork = bare_git(bare, &["merge-base", &origin_main, tip])
            .map_err(|why| format!("{tip} shares no history with origin/main: {why}"))?;
        let range = format!("{fork}..{tip}");
        let rev_list = bare_git(bare, &["rev-list", "--parents", &range])?;
        crate::api_push_history::linear(&rev_list, &fork)?;
        let diff =
            |format: &str| bare_git(bare, &["diff", "--no-renames", format, "-z", &fork, tip]);
        crate::api_push_checks::checked(&diff("--numstat")?, &diff("--raw")?, &self.allowed)
    }
}

/// Refuses to start the push when, `spent` into the call, the push and
/// the PR's deadlines could run past [`PUSH_CALL_DEADLINE`].
///
/// # Errors
///
/// Too little time left, saying nothing was pushed.
pub fn time_left_to_push(spent: Duration) -> Result<(), String> {
    let needed = crate::api_push_remote::PUSH_DEADLINE + 2 * crate::api_push_remote::GH_DEADLINE;
    if spent + needed > PUSH_CALL_DEADLINE {
        return Err(format!(
            "{spent:?} into the call, too little of its {PUSH_CALL_DEADLINE:?} is left to push and \
             open the PR ({needed:?}): nothing was pushed; call request_push again"
        ));
    }
    Ok(())
}

#[cfg(test)]
#[path = "api_push_tests.rs"]
mod tests;
