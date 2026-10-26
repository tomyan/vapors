use super::*;

#[test]
fn open_prs_are_read_oldest_first() {
    // Given: gh lists newest first
    let json = r#"[
      {"number":72,"headRefOid":"bbb","headRefName":"web/b","baseRefName":"main","isDraft":false,"isCrossRepository":false},
      {"number":68,"headRefOid":"aaa","headRefName":"dwim/a","baseRefName":"main","isDraft":true,"isCrossRepository":true}
    ]"#;

    // When
    let prs = parse_list(json).expect("parsed");

    // Then
    assert_eq!(prs[0].number, 68);
    assert!(prs[0].draft && prs[0].fork);
    assert_eq!(prs[1].branch, "web/b");
    assert_eq!(prs[1].base, "main");
}

#[test]
fn a_listed_pr_missing_a_field_is_refused_naming_it() {
    for (json, field) in [
        (
            r#"[{"number":68,"headRefOid":"aaa","baseRefName":"main","isDraft":false,"isCrossRepository":false}]"#,
            "headRefName",
        ),
        (
            r#"[{"number":68,"headRefOid":"aaa","headRefName":"b","baseRefName":"main","isDraft":false}]"#,
            "isCrossRepository",
        ),
    ] {
        let refused = parse_list(json).expect_err(field);
        assert!(
            refused.contains(field) && refused.contains("#68"),
            "{refused}"
        );
    }
}

#[test]
fn comment_pages_are_read_as_one_list() {
    let pages = "[\"one\",\"two\"]\n[\"three\"]\n";
    assert_eq!(
        parse_comment_pages(pages),
        Ok(vec!["one".into(), "two".into(), "three".into()])
    );
    assert!(parse_comment_pages("[1]").is_err());
}

#[test]
fn the_queue_never_pushes_main() {
    let refused =
        crate::queue_remote::push_lease(std::path::Path::new("/nonexistent"), "main", "a", "b")
            .expect_err("main");
    assert!(refused.contains("never pushes main"), "{refused}");
}

#[test]
fn the_push_skips_the_hook_and_leases_the_judged_head() {
    assert_eq!(
        crate::queue_remote::push_args("dwim/b", "h1", "h1new"),
        [
            "push",
            "-q",
            "--no-verify",
            "--force-with-lease=dwim/b:h1",
            "origin",
            "h1new:refs/heads/dwim/b"
        ]
    );
}

#[test]
fn the_gate_is_run_by_the_queue_s_own_binary_in_the_run_worktree() {
    let exe = std::path::PathBuf::from("/q/main/target/debug/xtask");
    let command = crate::queue_real::gate_command(&exe, std::path::Path::new("/q/run"), 71);
    assert_eq!(command.get_program(), exe.as_os_str());
    let args: Vec<_> = command
        .get_args()
        .map(|a| a.to_string_lossy().into_owned())
        .collect();
    assert_eq!(args, ["gate", "--pr", "71"]);
    assert_eq!(
        command.get_current_dir(),
        Some(std::path::Path::new("/q/run"))
    );
}
