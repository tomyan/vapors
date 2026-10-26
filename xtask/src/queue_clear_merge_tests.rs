//! Merge commits and the review clear, against real repositories.

use super::tests::lapsed;
use super::*;
use crate::queue_decide::Clear;
use crate::queue_git_fixture::{commit, lines_repo, run, try_run};

#[test]
fn a_merge_that_resolves_a_conflict_against_main_breaks_the_clear() {
    // Given: main changes the cleared line too; the PR merges main keeping its own side
    let (repo, cleared) = lines_repo("resolved");
    let text = std::fs::read_to_string(repo.0.join("f.txt")).expect("read");
    commit(&repo.0, "f.txt", &text.replace("line 5\n", "line MAIN\n"));
    run(&repo.0, &["checkout", "-q", "feature"]);
    run(
        &repo.0,
        &["merge", "-q", "-s", "ours", "main", "-m", "merge main"],
    );
    let head = run(&repo.0, &["rev-parse", "HEAD"]);

    // When
    let why = lapsed(clear_holds(&repo.0, "main", &head, &cleared));

    // Then
    assert_eq!(
        why,
        format!("merge {head} differs from git's own merge of its parents")
    );
}

#[test]
fn a_clean_merge_from_main_keeps_the_clear() {
    // Given: main changes line 1, far from the cleared line 5, and the PR merges it
    let (repo, cleared) = lines_repo("merged");
    let text = std::fs::read_to_string(repo.0.join("f.txt")).expect("read");
    commit(&repo.0, "f.txt", &text.replace("line 1\n", "line one\n"));
    run(&repo.0, &["checkout", "-q", "feature"]);
    run(&repo.0, &["merge", "-q", "main", "-m", "merge main"]);
    let head = run(&repo.0, &["rev-parse", "HEAD"]);

    // Then
    assert_eq!(
        clear_holds(&repo.0, "main", &head, &cleared),
        Ok(Clear::Holds)
    );
}

#[test]
fn a_merge_committed_with_its_conflict_markers_breaks_the_clear() {
    // Given: main changes the cleared line; the PR merges main and commits the conflict as it stands
    let (repo, cleared) = lines_repo("markers");
    let text = std::fs::read_to_string(repo.0.join("f.txt")).expect("read");
    commit(&repo.0, "f.txt", &text.replace("line 5\n", "line MAIN\n"));
    run(&repo.0, &["checkout", "-q", "feature"]);
    assert!(
        !try_run(&repo.0, &["merge", "-q", "main"]),
        "the merge conflicts"
    );
    run(
        &repo.0,
        &["commit", "-q", "-a", "-m", "merge main, markers and all"],
    );
    let head = run(&repo.0, &["rev-parse", "HEAD"]);

    // Then
    assert_eq!(
        lapsed(clear_holds(&repo.0, "main", &head, &cleared)),
        format!("merge {head} differs from git's own merge of its parents")
    );
}
