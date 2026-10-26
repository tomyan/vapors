//! Whether a review clear holds, against real repositories.

use super::*;
use crate::queue_decide::Clear;
use crate::queue_git_fixture::{commit, lines_repo, repo, run};

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
        lapsed(clear_holds(&repo.0, "main", &head, &cleared)),
        format!("cleared {cleared} is in neither the head nor main")
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
        lapsed(clear_holds(&repo.0, "main", &head, &cleared)),
        format!("{head} is not a patch the clear covers")
    );
}

/// The reason a clear lapsed, or a panic naming what came instead.
pub(super) fn lapsed(result: Result<Clear, String>) -> String {
    match result {
        Ok(Clear::Changed(why)) => why,
        other => panic!("expected the clear to lapse, got {other:?}"),
    }
}

/// A branch `name` from main with one commit writing `text` to f.txt.
fn branch_writing(dir: &std::path::Path, name: &str, text: &str) -> String {
    run(dir, &["checkout", "-q", "-b", name, "main"]);
    commit(dir, "f.txt", text)
}

#[test]
fn an_insertion_moved_elsewhere_in_its_file_breaks_the_clear() {
    // Given: the cleared commit inserts a call after line 5; the new one after line 9
    let (repo, _) = lines_repo("moved");
    let text = std::fs::read_to_string(repo.0.join("f.txt")).expect("read");
    let cleared = branch_writing(
        &repo.0,
        "cleared",
        &text.replace("line 5\n", "line 5\ncleanup\n"),
    );
    let head = branch_writing(
        &repo.0,
        "moved",
        &text.replace("line 9\n", "line 9\ncleanup\n"),
    );

    // When
    let why = lapsed(clear_holds(&repo.0, "main", &head, &cleared));

    // Then
    assert_eq!(why, format!("{head} is not a patch the clear covers"));
}

#[test]
fn the_same_insertion_added_twice_breaks_the_clear() {
    // Given: a file of identical lines, so both insertions have the same context and id
    let (repo, _) = lines_repo("twice");
    run(&repo.0, &["checkout", "-q", "main"]);
    let same = "x\n".repeat(8);
    commit(&repo.0, "f.txt", &same);
    let cleared = branch_writing(&repo.0, "once", &same.replacen("x\n", "x\nallow\n", 1));
    let twice = "x\nallow\nx\nx\nx\nallow\nx\nx\nx\nx\n";
    let head = commit(&repo.0, "f.txt", twice);

    // When
    let why = lapsed(clear_holds(&repo.0, "main", &head, &cleared));

    // Then
    assert_eq!(why, format!("{head} is not a patch the clear covers"));
}

#[test]
fn main_that_cannot_be_read_into_the_scratch_index_is_refused_naming_it() {
    // Given
    let (repo, cleared) = repo("unreadable");

    // When
    let why = in_main(&repo.0, "no-such-ref", &cleared).expect_err("refused");

    // Then
    assert!(why.starts_with("git read-tree no-such-ref: "), "{why}");
    assert!(why.contains("(index "), "{why}");
}
