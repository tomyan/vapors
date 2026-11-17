//! `request_push`, outside (`docs/plans/api-builder.md`, the lead's answer
//! 8): the builder's branch fetched into a bare repository
//! (`api_push_fetch.rs`), its diff checked there (`api_push_checks.rs`),
//! `cargo xtask ci` run on it in a fresh sandbox with the same fences,
//! then pushed from the bare repository and a PR opened. No git runs in
//! the builder's clone outside the sandbox.

use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::api_clone::ORIGIN_MAIN;
use crate::api_paths::Builder;
use crate::api_push_fetch::git as bare_git;
use crate::api_sandbox::Host;
use crate::deadline::{Finished, run_with_deadline};
use crate::queue_git::git;

/// `cargo xtask ci` in the fresh sandbox. Measured 2026-11-17: see the
/// commit that set it; three times the measured run.
pub const CI_DEADLINE: Duration = Duration::from_mins(40);

/// The advisory database's fetch on the host: the deny step's own
/// deadline for the same fetch (`steps.rs`).
const ADVISORY_DEADLINE: Duration = Duration::from_secs(60);

/// The sandboxed ci's command. Declared skip (the lead, L42): the host
/// tier (`host_` tests) runs bwrap and systemd, which the sandbox refuses
/// (`--disable-userns`, no user bus), so it cannot run inside; the
/// pre-push hook is skipped too (`pushed`). Retired when ci can run in a
/// sandbox the host tier works in.
pub const CI: [&str; 5] = ["cargo", "xtask", "ci", "--except", "host"];

/// What a push needs.
pub struct Pusher {
    pub main: PathBuf,
    pub builder: Builder,
    pub host: Host,
    /// The paths the branch may change (`api_push_checks::allowed`).
    pub allowed: Vec<String>,
}

impl Pusher {
    /// The branch checked, built, pushed, and its PR opened with `title`
    /// and `body`; what was done and the PR's URL.
    ///
    /// # Errors
    ///
    /// Any step refusing, its reason and output: the text the session reads.
    pub fn push(&self, title: &str, body: &str) -> Result<String, String> {
        let branch = crate::api_fences_run::branch(&self.builder);
        let bare = crate::api_push_fetch::fetched(&self.builder, &self.host.objects, &branch)?;
        let lines = self.checked(&bare, &branch)?;
        self.built(&bare, &branch)?;
        self.pushed(&bare, &branch)?;
        let url = self.opened(&branch, (title, body))?;
        Ok(format!("pushed {branch} ({lines} lines) and opened {url}"))
    }

    /// The branch's diff from where it left `origin/main`, checked; the
    /// lines it changes.
    fn checked(&self, bare: &Path, branch: &str) -> Result<u64, String> {
        let origin_main = git(&self.main, &["rev-parse", ORIGIN_MAIN])?;
        let fork = bare_git(bare, &["merge-base", &origin_main, branch])
            .map_err(|why| format!("{branch} shares no history with origin/main: {why}"))?;
        let numstat = bare_git(bare, &["diff", "--no-renames", "--numstat", &fork, branch])?;
        crate::api_push_checks::checked(&numstat, &self.allowed)
    }

    /// [`CI`] on the branch, in a fresh clone in a fresh sandbox, on
    /// the advisory database fetched just before.
    fn built(&self, bare: &Path, branch: &str) -> Result<(), String> {
        let host = self.advised()?;
        let ci = Builder::new(&format!("{}-ci", self.builder.name), &self.builder.home)?;
        let _ = std::fs::remove_dir_all(ci.root()); // the last request_push's; its target is kept
        let finished = self.ci_clone(&ci, (bare, branch)).and_then(|()| {
            let command = crate::api_fences_run::sandboxed(&ci, &host, &CI)?;
            run_with_deadline(command, CI_DEADLINE)
        });
        let _ = std::fs::remove_dir_all(ci.root());
        ci_passed(finished?)
    }

    /// `ci`'s clone, made from the main repository and moved to the
    /// branch's tip from the bare repository: host git runs in it only
    /// before its sandbox does.
    fn ci_clone(&self, ci: &Builder, (bare, branch): (&Path, &str)) -> Result<(), String> {
        crate::api_clone::make(ci, (&self.main, &self.host.objects), branch)?;
        let clone = ci.clone_dir();
        git(
            &clone,
            &["fetch", "--quiet", &bare.display().to_string(), branch],
        )?;
        git(&clone, &["checkout", "--quiet", "-B", branch, "FETCH_HEAD"]).map(|_| ())
    }

    /// The host with the advisory database cargo deny has just fetched
    /// in the main repository, which the sandbox, with no network, binds
    /// (`api_sandbox.rs` `Host::advisories`).
    fn advised(&self) -> Result<Host, String> {
        let mut fetch = std::process::Command::new("cargo");
        fetch.args(["deny", "fetch", "db"]).current_dir(&self.main);
        match run_with_deadline(fetch, ADVISORY_DEADLINE)? {
            Finished::Exited { success: true, .. } => {}
            Finished::Exited { output, .. } | Finished::TimedOut { output } => {
                return Err(format!(
                    "cargo deny fetch db failed on the host:\n{}",
                    tail(&output)
                ));
            }
        }
        let advisories = crate::api_host::advisories()?;
        if advisories.is_empty() {
            return Err(
                "cargo deny fetch db left no advisory database in ~/.cargo/advisory-dbs".into(),
            );
        }
        Ok(Host {
            advisories,
            ..self.host.clone()
        })
    }

    /// The branch pushed from the bare repository to the main
    /// repository's remote.
    fn pushed(&self, bare: &Path, branch: &str) -> Result<(), String> {
        let remote = git(&self.main, &["remote", "get-url", "origin"])?;
        let refspec = format!("refs/heads/{branch}:refs/heads/{branch}");
        // Relaxed guard, declared (the lead, answer 8): the pre-push hook
        // would run this branch's code (build scripts, tests, its xtask)
        // outside the sandbox, so it is skipped; the same checks ran as
        // [`CI`] in a fresh sandbox just before. Retired when the hook
        // can run its checks in a sandbox.
        let push = [
            "-c",
            "core.hooksPath=/dev/null",
            "push",
            "--no-verify",
            "--quiet",
            &remote,
            &refspec,
        ];
        git(bare, &push)
            .map(|_| ())
            .map_err(|why| format!("pushing {branch}: {why}"))
    }

    /// The PR for `branch`: the one already open, or a new one.
    fn opened(&self, branch: &str, (title, body): (&str, &str)) -> Result<String, String> {
        if let Ok(url) = gh(
            &self.main,
            &["pr", "view", branch, "--json", "url", "-q", ".url"],
        ) {
            return Ok(url);
        }
        let create = [
            "pr", "create", "--base", "main", "--head", branch, "--title", title, "--body", body,
        ];
        gh(&self.main, &create)
    }
}

/// How the sandboxed ci ended, as the session reads it.
fn ci_passed(finished: Finished) -> Result<(), String> {
    match finished {
        Finished::Exited { success: true, .. } => Ok(()),
        Finished::Exited { output, .. } => Err(format!(
            "{} failed in the sandbox:\n{}",
            CI.join(" "),
            tail(&output)
        )),
        Finished::TimedOut { output } => Err(format!(
            "{} passed its deadline of {CI_DEADLINE:?} in the sandbox:\n{}",
            CI.join(" "),
            tail(&output)
        )),
    }
}

fn gh(dir: &Path, args: &[&str]) -> Result<String, String> {
    let output = std::process::Command::new("gh")
        .args(args)
        .current_dir(dir)
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE")
        .output()
        .map_err(|e| format!("gh: {e}"))?;
    if output.status.success() {
        return Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned());
    }
    Err(format!(
        "gh {}: {}",
        args[..2].join(" "),
        String::from_utf8_lossy(&output.stderr).trim()
    ))
}

/// The last 60 lines of `output`: where ci names its failure.
fn tail(output: &str) -> String {
    let lines: Vec<&str> = output.lines().collect();
    lines[lines.len().saturating_sub(60)..].join("\n")
}
