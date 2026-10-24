//! The build's git stamp, shared by `build.rs` (which prints it for
//! cargo), the library (which the dist check compares against at check
//! time) and the tests (which run it on a temporary repository). The
//! short SHA, `-dirty` when the tree differs from HEAD, and the paths
//! whose change must rerun the build script.

use std::path::{Path, PathBuf};
use std::process::Command;

/// What `stamp` found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Stamp {
    /// `git rev-parse --short HEAD`, with `-dirty` when the tree differs.
    pub text: String,
    /// Files and directories a change of which must rerun the build
    /// script: HEAD, the ref HEAD resolves to, packed-refs, the index
    /// and every top-level tracked path.
    pub rerun: Vec<PathBuf>,
}

/// The stamp of the repository containing `dir`.
///
/// # Errors
///
/// A message naming the fix when git is missing or `dir` is not in a
/// checkout.
pub fn stamp(dir: &Path) -> Result<Stamp, String> {
    let sha = git(dir, &["rev-parse", "--short", "HEAD"])?;
    let dirty = !git(dir, &["status", "--porcelain"])?.is_empty();
    let text = if dirty { format!("{sha}-dirty") } else { sha };
    Ok(Stamp {
        text,
        rerun: rerun_paths(dir)?,
    })
}

fn rerun_paths(dir: &Path) -> Result<Vec<PathBuf>, String> {
    let git_path = |name: &str| git(dir, &["rev-parse", "--git-path", name]).map(PathBuf::from);
    let mut paths = vec![
        git_path("HEAD")?,
        git_path("packed-refs")?,
        git_path("index")?,
    ];
    let head_ref = git(dir, &["rev-parse", "--symbolic-full-name", "HEAD"])?;
    if head_ref.starts_with("refs/") {
        paths.push(git_path(&head_ref)?);
    }
    let tracked = git(dir, &["ls-files"])?;
    let mut top: Vec<PathBuf> = tracked
        .lines()
        .filter_map(|line| line.split('/').next())
        .map(PathBuf::from)
        .collect();
    top.sort();
    top.dedup();
    paths.extend(top);
    Ok(paths)
}

/// Runs git on `dir`'s repository and no other: git hooks export
/// `GIT_DIR`, `GIT_WORK_TREE` and `GIT_INDEX_FILE`, which would point every
/// call here at the hook's own repository (the pre-push hook running
/// the tests once committed a test repository onto the branch).
fn git(dir: &Path, args: &[&str]) -> Result<String, String> {
    let output = Command::new("git")
        .args(args)
        .current_dir(dir)
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE")
        .output()
        .map_err(|e| {
            format!(
                "git is needed to stamp the build ({e}); fix: install git and build from a checkout"
            )
        })?;
    if !output.status.success() {
        return Err(format!(
            "`git {}` failed: {}fix: build from a git checkout",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned())
}
