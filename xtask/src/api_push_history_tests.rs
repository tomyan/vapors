use super::linear;

#[test]
fn a_line_of_commits_from_the_fork_passes_with_its_count() {
    assert_eq!(linear("c3 c2\nc2 c1\nc1 fork\n", "fork"), Ok(3));
}

#[test]
fn a_merge_is_refused_naming_its_parents() {
    let refused = linear("c2 c1 orphan\nc1 fork\n", "fork").expect_err("refused");
    assert_eq!(
        refused,
        "c2 has 2 parents: push a line of commits, no merges"
    );
}

#[test]
fn a_parent_off_the_branch_is_refused() {
    let refused = linear("c1 elsewhere\n", "fork").expect_err("refused");
    assert!(
        refused.starts_with("c1's parent elsewhere is not on the branch"),
        "{refused}"
    );
}

#[test]
fn a_root_commit_or_nothing_to_push_is_refused() {
    assert!(
        linear("c1\n", "fork")
            .expect_err("root")
            .contains("0 parents")
    );
    assert!(
        linear("", "fork")
            .expect_err("empty")
            .starts_with("no commit after fork")
    );
}
