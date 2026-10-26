use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};

use super::*;

/// A directory of its own for one test, removed when dropped.
struct Scratch(PathBuf);

impl Scratch {
    fn new(name: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("xtask-targets-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("scratch");
        Self(dir)
    }

    /// `rel` under the scratch directory, created as a directory.
    fn dir(&self, rel: &str) -> PathBuf {
        let path = self.0.join(rel);
        std::fs::create_dir_all(&path).expect("dir");
        path
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn worktrees_on_a_branch_are_read_from_the_porcelain_list() {
    // Given
    let porcelain = "worktree /p/touche\nHEAD aaa\nbranch refs/heads/main\n\nworktree /w1/touche\nHEAD bbb\nbranch refs/heads/b1\n\nworktree /w10/touche\nHEAD ccc\nbranch refs/heads/b10\n\nworktree /m/touche\nHEAD bbb\ndetached\n\nworktree /w1b/touche\nHEAD bbb\nbranch refs/heads/b1\n";

    // When
    let found = worktrees_on(porcelain, "b1");

    // Then
    assert_eq!(
        found,
        [PathBuf::from("/w1/touche"), PathBuf::from("/w1b/touche")]
    );
}

#[test]
fn a_worktree_s_target_and_its_sibling_s_are_found_in_their_roots() {
    // Given: w/touche/target and w/vapors/target link into two roots
    let scratch = Scratch::new("found");
    let touche_root = scratch.dir("touche-targets");
    let vapors_root = scratch.dir("vapors-targets");
    let worktree = scratch.dir("w/touche");
    symlink(scratch.dir("touche-targets/x"), worktree.join("target")).expect("link");
    symlink(
        scratch.dir("vapors-targets/x"),
        scratch.dir("w/vapors").join("target"),
    )
    .expect("link");

    // When
    let mut dirs = target_dirs(&worktree, &[touche_root.clone(), vapors_root.clone()]);
    dirs.sort();

    // Then
    assert_eq!(dirs, [Ok(touche_root.join("x")), Ok(vapors_root.join("x"))]);
}

#[test]
fn a_target_that_is_a_directory_or_links_outside_the_roots_is_left_alone_naming_it() {
    // Given
    let scratch = Scratch::new("alone");
    let root = scratch.dir("touche-targets");
    let worktree = scratch.dir("w/touche");
    let real = scratch.dir("w/touche/target");
    let elsewhere = scratch.dir("elsewhere/x");
    symlink(&elsewhere, scratch.dir("w/vapors").join("target")).expect("link");

    // When
    let mut dirs = target_dirs(&worktree, &[root]);
    dirs.sort();

    // Then
    let link = scratch.0.join("w/vapors/target");
    assert_eq!(
        dirs,
        [
            Err(format!(
                "{} is a directory, not a link into a targets root; left alone",
                real.display()
            )),
            Err(format!(
                "{} links to {}, not a directory in a targets root; left alone",
                link.display(),
                elsewhere.display()
            )),
        ]
    );
}

#[test]
fn emptying_removes_the_contents_keeps_the_directory_and_follows_no_link() {
    // Given: a target with a file, a directory, and a link to a directory outside it
    let scratch = Scratch::new("empty");
    let target = scratch.dir("t/x");
    let outside = scratch.dir("outside");
    std::fs::write(outside.join("keep"), "kept").expect("file");
    std::fs::write(target.join("file"), "gone").expect("file");
    std::fs::write(scratch.dir("t/x/debug/deps").join("a"), "gone").expect("file");
    symlink(&outside, target.join("link")).expect("link");

    // When
    let removed = empty(&target);

    // Then
    assert_eq!(removed, Ok(3));
    assert_eq!(std::fs::read_dir(&target).expect("kept").count(), 0);
    assert!(Path::new(&outside.join("keep")).exists());
}
