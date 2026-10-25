//! The main check against a real repository and its clone.

use std::path::{Path, PathBuf};
use std::process::Command;

use super::*;

fn run_git(dir: &Path, args: &[&str]) {
    let status = Command::new("git")
        .args(args)
        .current_dir(dir)
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE")
        .env("GIT_AUTHOR_NAME", "t")
        .env("GIT_AUTHOR_EMAIL", "t@t")
        .env("GIT_COMMITTER_NAME", "t")
        .env("GIT_COMMITTER_EMAIL", "t@t")
        .status()
        .expect("git runs");
    assert!(status.success(), "git {args:?}");
}

/// A repository with one commit on main, and a clone of it.
fn origin_and_clone(name: &str) -> (PathBuf, PathBuf) {
    let root = std::env::temp_dir().join(format!("xtask-serve-git-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let origin = root.join("origin");
    std::fs::create_dir_all(&origin).expect("origin");
    run_git(&origin, &["init", "-q", "-b", "main"]);
    std::fs::write(origin.join("a.txt"), "a").expect("file");
    run_git(&origin, &["add", "."]);
    run_git(&origin, &["commit", "-q", "-m", "one"]);
    run_git(&root, &["clone", "-q", "origin", "clone"]);
    (origin, root.join("clone"))
}

#[test]
fn a_clean_clone_at_main_is_served_and_a_changed_one_is_not() {
    // Given
    let (_origin, clone) = origin_and_clone("clean");
    let dir = clone.display().to_string();

    // Then
    assert_eq!(require_main(&dir), Ok(()));
    std::fs::write(clone.join("a.txt"), "changed").expect("change");
    assert!(
        require_main(&dir)
            .expect_err("dirty")
            .contains("has changes")
    );
}

#[test]
fn a_clone_behind_main_is_refused_once_main_is_fetched() {
    // Given: main moves on after the clone
    let (origin, clone) = origin_and_clone("behind");
    std::fs::write(origin.join("b.txt"), "b").expect("file");
    run_git(&origin, &["add", "."]);
    run_git(&origin, &["commit", "-q", "-m", "two"]);
    let dir = clone.display().to_string();

    // When
    fetch_main(&dir).expect("fetched");

    // Then
    assert!(
        require_main(&dir)
            .expect_err("behind")
            .contains("not origin/main")
    );
}

#[test]
fn only_a_clean_tree_at_origin_main_passes() {
    assert_eq!(main_refusal("abc", "abc", false), None);
    assert!(main_refusal("abc", "def", false).is_some());
    assert!(main_refusal("abc", "abc", true).is_some());
}
