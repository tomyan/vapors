//! Re-publishing `/` after a merge: the serve worktree at origin/main,
//! its pinned siblings at their pins, then `cargo xtask serve --replace`
//! there. The xtask run is the serve worktree's, at main.

use std::path::Path;
use std::process::Command;
use std::time::Duration;

use crate::deadline::{Finished, run_with_deadline};

/// How long `serve --replace` may take: a release build of the page and
/// its publish. Chosen until measured: a release `cargo xtask web` took
/// about 1 to 2 min on the box under load.
const SERVE_DEADLINE: Duration = Duration::from_mins(20);

/// Re-publish `/` from `dir`.
///
/// # Errors
///
/// A step's refusal, or the publish not finishing in time.
pub fn serve(dir: &Path) -> Result<(), String> {
    crate::queue_remote::fetch(dir, &["main"])?;
    crate::queue_git::git(dir, &["checkout", "-q", "--detach", "origin/main"])?;
    crate::queue_pins::move_siblings(dir)?;
    let mut command = Command::new("cargo");
    command
        .args(["xtask", "serve", "--replace"])
        .current_dir(dir);
    match run_with_deadline(command, SERVE_DEADLINE)? {
        Finished::Exited { success: true, .. } => Ok(()),
        Finished::Exited { output, .. } => Err(format!(
            "serve --replace failed: {}",
            crate::gate_pr::tail(&output, 5)
        )),
        Finished::TimedOut { .. } => Err(format!(
            "serve --replace did not finish within {SERVE_DEADLINE:?}"
        )),
    }
}
