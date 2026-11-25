//! `request_push`'s `cargo xtask ci`, in a fresh clone in a fresh sandbox
//! with the same fences as the session's, on the advisory database the
//! host has just fetched. The ci builder is this call's own
//! (`Builder::ci`): its clone, target and cargo home are new, and its
//! sccache is the ci runs', not the sessions'.
//!
//! The verdict is advisory (review of slice 4, M2): code on the branch
//! runs inside, and a build script or test can make ci pass whatever it
//! does (`std::process::exit(0)` first). The guard is the merge gate,
//! which runs everything again on the host before any merge.

use std::path::Path;
use std::time::Duration;

use crate::api_paths::Builder;
use crate::api_push::Pusher;
use crate::api_push_fetch::git;
use crate::api_sandbox::Host;
use crate::deadline::{Finished, run_with_deadline};

/// `cargo xtask ci` in the fresh sandbox: three times the measured run
/// (the lead, L43). Measured 2026-11-17 on main plus slice 4, through
/// `sandboxed` with [`CI`], in a new ci builder (cold target): 331 s,
/// every step passing, the ci sccache warm, load 6 to 21. A run with the
/// ci sccache cold took 585 s to its first failure, 83% of the way
/// through the unit tier (the failure since fixed); a whole cold run is
/// about 700 s, inside this with room for load. 3 x 331 s is 993 s.
pub const CI_DEADLINE: Duration = Duration::from_mins(17);

/// The advisory database's fetch on the host: the deny step's own
/// deadline for the same fetch (`steps.rs`).
const ADVISORY_DEADLINE: Duration = Duration::from_secs(60);

/// The sandboxed ci's command. Declared skip (the lead, L42): the host
/// tier (`host_` tests) runs bwrap where there is a render node, which the
/// sandbox refuses (`--disable-userns`), so it cannot run inside. Retired
/// when ci can run in a sandbox the host tier works in.
pub const CI: [&str; 5] = ["cargo", "xtask", "ci", "--except", "host"];

/// [`CI`] on `tip`, in call `call`'s clone and sandbox, removed after.
///
/// # Errors
///
/// The advisory fetch, the clone or ci failing: the text the session
/// reads, with ci's last lines.
pub fn built(pusher: &Pusher, (bare, tip): (&Path, &str), call: u64) -> Result<(), String> {
    let host = advised(pusher)?;
    let ci = pusher.builder.ci(call);
    let finished = ci_clone(&ci, (&host.objects, bare, tip)).and_then(|()| {
        let command = crate::api_fences_run::sandboxed(&ci, &host, &CI)?;
        run_with_deadline(command, CI_DEADLINE)
    });
    for made in [ci.root(), ci.target()] {
        let _ = std::fs::remove_dir_all(made); // this call's; the verdict is the result
    }
    passed(finished?)
}

/// `ci`'s clone: a new repository reading the main objects through
/// alternates, `tip` fetched from `bare` and checked out on the branch,
/// every git call in a cleared environment with a deadline (review of
/// slice 4, L2, M3), then the directories the sandbox binds.
fn ci_clone(ci: &Builder, (objects, bare, tip): (&Path, &Path, &str)) -> Result<(), String> {
    if ci.root().exists() {
        return Err(format!(
            "{} is already there: each push's ci starts new",
            ci.root().display()
        ));
    }
    let clone = ci.clone_dir();
    std::fs::create_dir_all(&clone).map_err(|e| format!("making {}: {e}", clone.display()))?;
    git(&clone, &["init", "--quiet"])?;
    let alternates = clone.join(".git/objects/info/alternates");
    std::fs::write(&alternates, format!("{}\n", objects.display()))
        .map_err(|e| format!("writing {}: {e}", alternates.display()))?;
    let branch = crate::api_fences_run::branch(ci);
    git(
        &clone,
        &[
            "fetch",
            "--quiet",
            "--no-tags",
            &bare.display().to_string(),
            tip,
        ],
    )?;
    git(&clone, &["checkout", "--quiet", "-b", &branch, tip])?;
    crate::api_clone::directories(ci)
}

/// The host with the advisory database cargo deny has just fetched
/// in the main repository, which the sandbox, with no network, binds
/// (`api_sandbox.rs` `Host::advisories`).
fn advised(pusher: &Pusher) -> Result<Host, String> {
    let mut fetch = std::process::Command::new("cargo");
    fetch
        .args(["deny", "fetch", "db"])
        .current_dir(&pusher.main);
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
        ..pusher.host.clone()
    })
}

/// How the sandboxed ci ended, as the session reads it.
fn passed(finished: Finished) -> Result<(), String> {
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

/// The last 60 lines of `output`: where ci names its failure.
fn tail(output: &str) -> String {
    let lines: Vec<&str> = output.lines().collect();
    lines[lines.len().saturating_sub(60)..].join("\n")
}
