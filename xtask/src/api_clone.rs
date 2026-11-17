//! An API builder's clone (`docs/plans/api-builder.md`): its own
//! repository, not a worktree, so the sandbox binds no git directory
//! another branch uses. Its objects come from the main repository through
//! alternates, bound read-only inside; its branch starts at the main
//! repository's `origin/main`, fetched first; its `origin` is the main
//! repository's remote, which the sandbox cannot reach. The harness
//! fetches the branch back from outside to push it.

use std::fs;
use std::os::unix::fs::{PermissionsExt, symlink};
use std::path::{Path, PathBuf};

use crate::api_paths::Builder;
use crate::queue_git::git;

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

/// `builder`'s clone of `main` on a new branch `branch` at
/// `origin/main`, its target directory linked, its cargo home and socket
/// directory made.
///
/// # Errors
///
/// A builder of that name already there (its directory names the fix),
/// or git or the filesystem failing, naming the step.
pub fn make(builder: &Builder, main: &Path, branch: &str) -> Result<(), String> {
    if builder.root().exists() {
        return Err(format!(
            "builder {} already has {}: choose another name, or remove it once its run is done",
            builder.name,
            builder.root().display()
        ));
    }
    git(main, &["fetch", "--quiet", "origin"])?;
    cloned(builder, main, branch)?;
    directories(builder)
}

fn cloned(builder: &Builder, main: &Path, branch: &str) -> Result<(), String> {
    let clone = builder.clone_dir();
    let clone_text = clone.display().to_string();
    let main_text = main.display().to_string();
    git(
        main,
        &[
            "clone",
            "--quiet",
            "--shared",
            "--no-checkout",
            &main_text,
            &clone_text,
        ],
    )?;
    let remote = git(main, &["remote", "get-url", "origin"])?;
    git(
        &clone,
        &["fetch", "--quiet", &main_text, "refs/remotes/origin/main"],
    )?;
    git(&clone, &["checkout", "--quiet", "-b", branch, "FETCH_HEAD"])?;
    git(&clone, &["remote", "set-url", "origin", &remote])?;
    Ok(())
}

fn directories(builder: &Builder) -> Result<(), String> {
    let made = |path: &Path| {
        fs::create_dir_all(path).map_err(|e| format!("making {}: {e}", path.display()))
    };
    made(&builder.target())?;
    made(&builder.cargo_home().join("registry"))?;
    made(&builder.socket_dir())?;
    let private = fs::Permissions::from_mode(0o700);
    fs::set_permissions(builder.socket_dir(), private)
        .map_err(|e| format!("making {} private: {e}", builder.socket_dir().display()))?;
    let link = builder.clone_dir().join("target");
    symlink(builder.target(), &link).map_err(|e| format!("linking {}: {e}", link.display()))
}
