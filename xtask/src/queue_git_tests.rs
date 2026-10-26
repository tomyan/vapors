//! The queue's git questions against real repositories, each removed
//! when its guard drops.

use std::path::{Path, PathBuf};
use std::process::Command;

use super::*;
use crate::queue_decide::Clear;

struct Repo(PathBuf);

impl Drop for Repo {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn run(dir: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .args(args)
        .current_dir(dir)
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE")
        .env("GIT_AUTHOR_NAME", "t")
        .env("GIT_AUTHOR_EMAIL", "t@t")
        .env("GIT_COMMITTER_NAME", "t")
        .env("GIT_COMMITTER_EMAIL", "t@t")
        .output()
        .expect("git runs");
    assert!(
        output.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).trim().to_owned()
}

fn commit(dir: &Path, file: &str, text: &str) -> String {
    std::fs::write(dir.join(file), text).expect("file");
    run(dir, &["add", "."]);
    run(dir, &["commit", "-q", "-m", file]);
    run(dir, &["rev-parse", "HEAD"])
}

/// main with one commit, and `feature` branched from it with two.
fn repo(name: &str) -> (Repo, String) {
    let dir = std::env::temp_dir().join(format!("xtask-queue-git-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("dir");
    run(&dir, &["init", "-q", "-b", "main"]);
    commit(&dir, "base.txt", "base");
    run(&dir, &["checkout", "-q", "-b", "feature"]);
    commit(&dir, "a.txt", "a");
    let head = commit(&dir, "b.txt", "b");
    run(&dir, &["checkout", "-q", "main"]);
    (Repo(dir), head)
}

#[test]
fn a_clean_rebase_keeps_the_review_clear() {
    // Given: cleared at the feature's head, then main moves and it rebases
    let (repo, cleared) = repo("clear");
    commit(&repo.0, "main.txt", "main");
    run(&repo.0, &["checkout", "-q", "feature"]);
    run(&repo.0, &["rebase", "-q", "main"]);
    let head = run(&repo.0, &["rev-parse", "HEAD"]);

    // Then
    assert_ne!(head, cleared);
    assert_eq!(
        clear_holds(&repo.0, "main", &head, &cleared),
        Ok(Clear::Holds)
    );
}

/// main with a ten-line file, and `feature` changing its line 5.
fn lines_repo(name: &str) -> (Repo, String) {
    let dir = std::env::temp_dir().join(format!("xtask-queue-lines-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("dir");
    run(&dir, &["init", "-q", "-b", "main"]);
    let lines: Vec<String> = (1..=10).map(|n| format!("line {n}")).collect();
    let text = lines.join("\n") + "\n";
    commit(&dir, "f.txt", &text);
    run(&dir, &["checkout", "-q", "-b", "feature"]);
    let head = commit(&dir, "f.txt", &text.replace("line 5\n", "line five\n"));
    run(&dir, &["checkout", "-q", "main"]);
    (Repo(dir), head)
}

#[test]
fn a_clean_rebase_onto_a_change_two_lines_from_a_hunk_keeps_the_clear() {
    // Given: main changes line 3, two lines from the feature's line 5
    let (repo, cleared) = lines_repo("context");
    let text = std::fs::read_to_string(repo.0.join("f.txt")).expect("read");
    commit(&repo.0, "f.txt", &text.replace("line 3\n", "line three\n"));
    run(&repo.0, &["checkout", "-q", "feature"]);
    run(&repo.0, &["rebase", "-q", "main"]);
    let head = run(&repo.0, &["rev-parse", "HEAD"]);

    // Then
    assert_eq!(
        clear_holds(&repo.0, "main", &head, &cleared),
        Ok(Clear::Holds)
    );
}

#[test]
fn a_stacked_pr_keeps_its_clear_once_its_parent_is_squashed_into_main() {
    // Given: parent adds a.txt; the PR, on the parent, adds b.txt; cleared
    let (repo, _) = repo("stacked");
    run(&repo.0, &["checkout", "-q", "-b", "child", "feature"]);
    let cleared = commit(&repo.0, "c.txt", "c");
    // The parent (feature) is squash-merged into main.
    run(&repo.0, &["checkout", "-q", "main"]);
    run(&repo.0, &["merge", "-q", "--squash", "feature"]);
    run(&repo.0, &["commit", "-q", "-m", "squash of feature"]);
    // The PR is rebased onto main: its parent's commits drop as applied.
    run(&repo.0, &["checkout", "-q", "child"]);
    run(&repo.0, &["rebase", "-q", "main"]);
    let head = run(&repo.0, &["rev-parse", "HEAD"]);

    // Then
    assert_eq!(
        clear_holds(&repo.0, "main", &head, &cleared),
        Ok(Clear::Holds)
    );
}

#[test]
fn a_cleared_commit_dropped_but_not_in_main_breaks_the_clear() {
    // Given: cleared with a.txt and b.txt; then b.txt's commit is dropped
    let (repo, cleared) = repo("dropped");
    run(&repo.0, &["checkout", "-q", "feature"]);
    run(&repo.0, &["reset", "-q", "--hard", "HEAD~1"]);
    let head = run(&repo.0, &["rev-parse", "HEAD"]);

    // Then
    assert_eq!(
        clear_holds(&repo.0, "main", &head, &cleared),
        Ok(Clear::Changed)
    );
}

#[test]
fn a_changed_commit_breaks_the_review_clear() {
    // Given: cleared, then a commit's content changes
    let (repo, cleared) = repo("changed");
    run(&repo.0, &["checkout", "-q", "feature"]);
    std::fs::write(repo.0.join("b.txt"), "b, fixed").expect("file");
    run(&repo.0, &["commit", "-q", "-a", "--amend", "--no-edit"]);
    let head = run(&repo.0, &["rev-parse", "HEAD"]);

    // Then
    assert_eq!(
        clear_holds(&repo.0, "main", &head, &cleared),
        Ok(Clear::Changed)
    );
}

#[test]
fn a_head_contains_main_only_once_rebased_onto_it() {
    // Given
    let (repo, head) = repo("contains");
    assert_eq!(contains(&repo.0, "main", &head), Ok(true));
    commit(&repo.0, "main.txt", "main");

    // Then
    assert_eq!(contains(&repo.0, "main", &head), Ok(false));
}

#[test]
fn a_rebase_that_conflicts_is_unclean_and_aborted() {
    // Given: main changes the file the feature changed
    let (repo, head) = repo("conflict");
    commit(&repo.0, "b.txt", "main's b");

    // When
    let clean = rebase_clean(&repo.0, "main", &head);

    // Then
    assert_eq!(clean, Ok(None));
    assert_eq!(
        run(&repo.0, &["status", "--porcelain"]),
        "",
        "the rebase was aborted"
    );
}

#[test]
fn a_clean_rebase_gives_the_new_head() {
    // Given
    let (repo, head) = repo("rebased");
    let main = commit(&repo.0, "main.txt", "main");

    // When
    let rebased = rebase_clean(&repo.0, "main", &head)
        .expect("ran")
        .expect("clean");

    // Then
    assert_ne!(rebased, head);
    assert_eq!(contains(&repo.0, &main, &rebased), Ok(true));
}
