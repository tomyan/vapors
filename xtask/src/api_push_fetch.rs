//! The way back from an untrusted clone (`docs/plans/api-builder.md`,
//! "The clone is untrusted"): the builder's branch fetched into a bare
//! repository made for the push, whose objects come from the main
//! repository's through alternates. git runs in the bare repository,
//! never in the clone, with hooks off and a cleared environment, so
//! nothing the session wrote into the clone's config, hooks or files
//! runs on the host. The diff is read, and the push made, from here.

use std::path::{Path, PathBuf};
use std::process::Command;

use crate::api_paths::Builder;

/// The variables git keeps: where to find itself, and no system or user
/// config, so no `core.hooksPath` or helper from either applies.
const GIT_ENV: [(&str, &str); 3] = [
    ("PATH", "/usr/bin:/bin"),
    ("GIT_CONFIG_NOSYSTEM", "1"),
    ("GIT_CONFIG_GLOBAL", "/dev/null"),
];

/// A fresh bare repository at `builder`'s `push.git`, its objects the
/// main repository's (`objects`) through alternates, and `branch` of the
/// builder's clone fetched into it; its path.
///
/// # Errors
///
/// git failing, naming the step; a branch not in the clone says to
/// commit on it.
pub fn fetched(builder: &Builder, objects: &Path, branch: &str) -> Result<PathBuf, String> {
    let bare = builder.root().join("push.git");
    let _ = std::fs::remove_dir_all(&bare); // the last request_push's, if any
    git(Path::new("/"), &["init", "--quiet", "--bare", &text(&bare)])?;
    let alternates = bare.join("objects/info/alternates");
    std::fs::write(&alternates, format!("{}\n", objects.display()))
        .map_err(|e| format!("writing {}: {e}", alternates.display()))?;
    let refspec = format!("+refs/heads/{branch}:refs/heads/{branch}");
    let clone = text(&builder.clone_dir());
    let fetch = [
        "fetch",
        "--quiet",
        "--no-tags",
        "--no-recurse-submodules",
        &clone,
        &refspec,
    ];
    git(&bare, &fetch).map_err(|why| {
        format!("fetching branch {branch} from the clone: {why}; commit your work on that branch")
    })?;
    Ok(bare)
}

/// git in `dir` (never the clone), hooks off, `file://` allowed for the
/// clone, its environment cleared to [`GIT_ENV`]; stdout trimmed.
///
/// # Errors
///
/// git not starting or not exiting 0, with its stderr.
pub fn git(dir: &Path, args: &[&str]) -> Result<String, String> {
    let mut command = Command::new("git");
    command.env_clear().envs(GIT_ENV).current_dir(dir);
    command.args([
        "-c",
        "core.hooksPath=/dev/null",
        "-c",
        "protocol.file.allow=always",
    ]);
    let output = command
        .args(args)
        .output()
        .map_err(|e| format!("git: {e}"))?;
    if !output.status.success() {
        return Err(crate::queue_git::failure(args, &output));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

fn text(path: &Path) -> String {
    path.display().to_string()
}

#[cfg(test)]
#[path = "api_push_fetch_tests.rs"]
mod tests;
