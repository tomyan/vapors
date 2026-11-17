//! An API builder's clone (`docs/plans/api-builder.md`): its own
//! repository, not a worktree, so the sandbox binds no git directory
//! another branch uses. Its objects come from the main repository through
//! alternates, bound read-only inside; its one ref is the main
//! repository's `origin/main`, fetched first, and its branch starts
//! there; its `origin` is the main repository's remote, which the sandbox
//! cannot reach. Once a session has run in it the clone is untrusted:
//! the host runs no git or cargo in it (the plan's "The clone is
//! untrusted").

use std::fs;
use std::os::unix::fs::{PermissionsExt, symlink};
use std::path::{Path, PathBuf};

use crate::api_paths::Builder;
use crate::queue_git::git;

/// The one ref of the main repository's a clone holds.
pub const ORIGIN_MAIN: &str = "refs/remotes/origin/main";

/// The main repository this xtask runs in: its top directory and its
/// object directory.
///
/// # Errors
///
/// git failing in the current directory.
pub fn main_repository() -> Result<(PathBuf, PathBuf), String> {
    let common = git(
        Path::new("."),
        &["rev-parse", "--path-format=absolute", "--git-common-dir"],
    )?;
    let common = PathBuf::from(common);
    let top = common.parent().map(Path::to_owned).ok_or_else(|| {
        format!(
            "the git common directory {} has no parent",
            common.display()
        )
    })?;
    Ok((top, common.join("objects")))
}

/// `builder`'s clone of `main` (whose object directory is `objects`) on
/// a new branch `branch` at `origin/main`, its target directory linked,
/// its cargo home and socket directory made; on a failure, what it made
/// is removed.
///
/// # Errors
///
/// A builder of that name already there (its directory names the fix),
/// or git or the filesystem failing, naming the step.
pub fn make(
    builder: &Builder,
    (main, objects): (&Path, &Path),
    branch: &str,
) -> Result<(), String> {
    if builder.root().exists() {
        return Err(format!(
            "builder {} already has {}: choose another name, or remove it once its run is done",
            builder.name,
            builder.root().display()
        ));
    }
    git(main, &["fetch", "--quiet", "origin"])?;
    let made = cloned(builder, (main, objects), branch).and_then(|()| directories(builder));
    if made.is_err() {
        for left in [builder.root(), builder.target()] {
            let _ = fs::remove_dir_all(left); // what this run made; the failure is the error
        }
    }
    made
}

/// A new repository whose objects are `objects`, read through
/// alternates, holding one ref of the main repository's,
/// `origin/main`: no other branch, pushed or not, is in it (review of
/// #467, M2).
fn cloned(builder: &Builder, (main, objects): (&Path, &Path), branch: &str) -> Result<(), String> {
    let clone = builder.clone_dir();
    git(main, &["init", "--quiet", &clone.display().to_string()])?;
    let alternates = clone.join(".git/objects/info/alternates");
    fs::write(&alternates, format!("{}\n", objects.display()))
        .map_err(|e| format!("writing {}: {e}", alternates.display()))?;
    let source = main.display().to_string();
    let main_ref = format!("+{ORIGIN_MAIN}:{ORIGIN_MAIN}");
    git(
        &clone,
        &["fetch", "--quiet", "--no-tags", &source, &main_ref],
    )?;
    git(&clone, &["checkout", "--quiet", "-b", branch, ORIGIN_MAIN])?;
    let remote = git(main, &["remote", "get-url", "origin"])?;
    git(&clone, &["remote", "add", "origin", &remote])?;
    Ok(())
}

/// `builder`'s target, sccache, cargo home and socket directories made,
/// and its target linked into its clone.
///
/// # Errors
///
/// The filesystem failing, naming the path.
pub fn directories(builder: &Builder) -> Result<(), String> {
    let made = |path: &Path| {
        fs::create_dir_all(path).map_err(|e| format!("making {}: {e}", path.display()))
    };
    made(&builder.target())?;
    made(&builder.sandbox_sccache())?;
    made(&builder.cargo_home().join("registry"))?;
    made(&builder.socket_dir())?;
    let private = fs::Permissions::from_mode(0o700);
    fs::set_permissions(builder.socket_dir(), private)
        .map_err(|e| format!("making {} private: {e}", builder.socket_dir().display()))?;
    let link = builder.clone_dir().join("target");
    symlink(builder.target(), &link).map_err(|e| format!("linking {}: {e}", link.display()))
}

#[cfg(test)]
#[path = "api_clone_tests.rs"]
mod tests;
