//! The queue's git questions against real repositories.

use super::*;
use crate::queue_git_fixture::{commit, repo, run};

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

#[test]
fn a_rename_lists_both_its_paths_as_changed() {
    // Given: the feature moves a file out of xtask/
    let (repo, _) = repo("renamed");
    std::fs::create_dir_all(repo.0.join("xtask")).expect("dir");
    commit(&repo.0, "xtask/queue-pass.sh", "#!/bin/sh\necho pass\n");
    run(&repo.0, &["checkout", "-q", "-b", "moved"]);
    run(&repo.0, &["mv", "xtask/queue-pass.sh", "queue-pass.sh"]);
    run(&repo.0, &["commit", "-q", "-m", "move"]);
    let head = run(&repo.0, &["rev-parse", "HEAD"]);

    // When
    let changed = changed_paths(&repo.0, "main", &head);

    // Then
    assert_eq!(
        changed,
        Ok(vec![
            "queue-pass.sh".to_owned(),
            "xtask/queue-pass.sh".to_owned()
        ])
    );
}
