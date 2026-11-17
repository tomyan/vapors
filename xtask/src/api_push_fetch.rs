//! The way back from an untrusted clone (`docs/plans/api-builder.md`,
//! "The clone is untrusted"): the builder's branch fetched into a bare
//! repository made for the push, whose objects come from the main
//! repository's through alternates. git runs in the bare repository,
//! never in the clone, with hooks off and a cleared environment, so
//! nothing the session wrote into the clone's config, hooks or files
//! runs on the host. The diff is read, and the push made, from here.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use crate::api_paths::Builder;

/// The variables git keeps: where to find itself, and no system or user
/// config, so no `core.hooksPath` or helper from either applies.
const GIT_ENV: [(&str, &str); 3] = [
    ("PATH", "/usr/bin:/bin"),
    ("GIT_CONFIG_NOSYSTEM", "1"),
    ("GIT_CONFIG_GLOBAL", "/dev/null"),
];

/// The most a local git call here may take: on this host the tests'
/// init, fetch from a clone, merge-base and diff each take under 0.1 s
/// (2026-11-17); 60 s covers a loaded host and a slice's whole pack.
pub const LOCAL_GIT_DEADLINE: Duration = Duration::from_secs(60);

/// A new bare repository for push number `call`, its objects the main
/// repository's (`objects`) through alternates, and `branch` of the
/// builder's clone fetched into it, every object checked
/// (`transfer.fsckObjects`); its path. Each call has its own, so one
/// push's objects are never another's (review of slice 4, H1).
///
/// # Errors
///
/// The clone's repository not as `api_clone` made it, the bare
/// repository already there, or git failing, naming the step; a branch
/// not in the clone says to commit on it.
pub fn fetched(
    builder: &Builder,
    objects: &Path,
    (branch, call): (&str, u64),
) -> Result<PathBuf, String> {
    clone_as_made(&builder.clone_dir(), objects)?;
    let bare = builder.root().join(format!("push-{call}.git"));
    if bare.exists() {
        return Err(format!(
            "{} is already there: each push starts in a new repository",
            bare.display()
        ));
    }
    git(Path::new("/"), &["init", "--quiet", "--bare", &text(&bare)])?;
    let alternates = bare.join("objects/info/alternates");
    std::fs::write(&alternates, format!("{}\n", objects.display()))
        .map_err(|e| format!("writing {}: {e}", alternates.display()))?;
    let refspec = format!("+refs/heads/{branch}:refs/heads/{branch}");
    let clone = text(&builder.clone_dir());
    let fetch = [
        "-c",
        "transfer.fsckObjects=true",
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

/// Refuses a clone whose repository the session has re-pointed: the host's
/// upload-pack follows `.git` as a gitfile, symlinked object directories
/// and alternates, any of which could reach another repository's objects
/// on the host (review of slice 4, M1). Checked without following links.
fn clone_as_made(clone: &Path, objects: &Path) -> Result<(), String> {
    let git_dir = clone.join(".git");
    for (path, what) in [
        (git_dir.clone(), "a directory"),
        (git_dir.join("objects"), "a directory"),
        (git_dir.join("objects/pack"), "a directory"),
    ] {
        let is_dir = std::fs::symlink_metadata(&path).is_ok_and(|meta| meta.file_type().is_dir());
        if !is_dir {
            return Err(format!(
                "{} is not {what} (a link or a file): the clone was changed; nothing is fetched",
                path.display()
            ));
        }
    }
    let alternates = git_dir.join("objects/info/alternates");
    let found = std::fs::read_to_string(&alternates)
        .map_err(|e| format!("reading {}: {e}", alternates.display()))?;
    if found != format!("{}\n", objects.display()) {
        return Err(format!(
            "{} names {found:?}, not the main repository's objects: nothing is fetched",
            alternates.display()
        ));
    }
    Ok(())
}

/// git in `dir` (never the clone), hooks off, `file://` allowed for the
/// clone, its environment cleared to [`GIT_ENV`], within
/// [`LOCAL_GIT_DEADLINE`]; stdout trimmed.
///
/// # Errors
///
/// git not starting, passing its deadline, or not exiting 0, with its
/// stderr.
pub fn git(dir: &Path, args: &[&str]) -> Result<String, String> {
    let mut command = Command::new("git");
    command.env_clear().envs(GIT_ENV).current_dir(dir);
    command.args([
        "-c",
        "core.hooksPath=/dev/null",
        "-c",
        "protocol.file.allow=always",
    ]);
    command.args(args);
    let ended = crate::api_timed::within(command, LOCAL_GIT_DEADLINE)?;
    if !ended.status.success() {
        return Err(format!(
            "git {}: {}",
            args.join(" "),
            String::from_utf8_lossy(&ended.stderr).trim()
        ));
    }
    Ok(String::from_utf8_lossy(&ended.stdout).trim().to_owned())
}

fn text(path: &Path) -> String {
    path.display().to_string()
}

#[cfg(test)]
#[path = "api_push_fetch_tests.rs"]
mod tests;
