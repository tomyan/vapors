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
    let upload_pack = upload_pack(&builder.clone_dir(), objects)?;
    let fetch = [
        "-c",
        "transfer.fsckObjects=true",
        "fetch",
        "--upload-pack",
        &upload_pack,
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

/// The upload-pack the fetch runs: inside bwrap, seeing only the clone
/// and the main repository's objects, read-only, with no network and
/// nothing else of the host (verify of slice 4, V1). The bind exposes
/// every object in the main repository's store, the set the session can
/// already read through its alternates (`api_clone.rs` `cloned`); what is
/// pushed is governed by the per-commit checks (re-verify, R2, accepted
/// by the lead, L56). A link the session
/// planted anywhere in the clone's `.git` (a fan-out directory, a pack, a
/// gitfile, alternates), even one swapped in after `clone_as_made`, names
/// nothing inside, so no other repository's objects can be read. The
/// clone's directory is bound, not its `.git`, as the session cannot
/// replace the directory itself; git runs it through `sh -c`, so each
/// word is quoted.
///
/// # Errors
///
/// A path holding a `'`, which the quoting cannot carry.
fn upload_pack(clone: &Path, objects: &Path) -> Result<String, String> {
    let mut words: Vec<String> = [
        "bwrap",
        "--unshare-all",
        "--die-with-parent",
        "--new-session",
        "--clearenv",
        "--setenv",
        "PATH",
        "/usr/bin:/bin",
        "--ro-bind",
        "/usr",
        "/usr",
        "--symlink",
        "usr/bin",
        "/bin",
        "--symlink",
        "usr/lib",
        "/lib",
        "--symlink",
        "usr/lib64",
        "/lib64",
        "--proc",
        "/proc",
        "--dev",
        "/dev",
    ]
    .map(str::to_owned)
    .to_vec();
    for bound in [clone, objects] {
        words.extend(["--ro-bind".to_owned(), text(bound), text(bound)]);
    }
    words.push("git-upload-pack".to_owned());
    if let Some(word) = words.iter().find(|word| word.contains('\'')) {
        return Err(format!(
            "{word:?} holds a quote: request_push cannot fetch from it"
        ));
    }
    Ok(words
        .iter()
        .map(|word| format!("'{word}'"))
        .collect::<Vec<_>>()
        .join(" "))
}

/// Refuses a clone whose repository the session has re-pointed (review of
/// slice 4, M1), checked without following links: a first check, before
/// the sandboxed upload-pack (`upload_pack`), which is the guard.
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
    let found = alternates_of(&alternates)?;
    if found != format!("{}\n", objects.display()) {
        return Err(format!(
            "{} names {found:?}, not the main repository's objects: nothing is fetched",
            alternates.display()
        ));
    }
    Ok(())
}

/// The alternates file read without following a link, and at most
/// [`ALTERNATES_BYTES`]: a FIFO, `/dev/zero` or a link to either is
/// refused, not read (verify of slice 4, V3).
fn alternates_of(path: &Path) -> Result<String, String> {
    let meta =
        std::fs::symlink_metadata(path).map_err(|e| format!("reading {}: {e}", path.display()))?;
    if !meta.file_type().is_file() || meta.len() > ALTERNATES_BYTES {
        return Err(format!(
            "{} is not a file of at most {ALTERNATES_BYTES} bytes: nothing is fetched",
            path.display()
        ));
    }
    let mut found = String::new();
    let file = std::fs::File::open(path).map_err(|e| format!("reading {}: {e}", path.display()))?;
    std::io::Read::read_to_string(&mut std::io::Read::take(file, ALTERNATES_BYTES), &mut found)
        .map_err(|e| format!("reading {}: {e}", path.display()))?;
    Ok(found)
}

/// The most of an alternates file read: `api_clone` writes one path.
const ALTERNATES_BYTES: u64 = 4096;

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
