use super::*;

#[test]
fn open_prs_are_read_oldest_first_with_their_comments() {
    // Given: gh lists newest first
    let json = r#"[
      {"number":72,"headRefOid":"bbb","headRefName":"web/b","baseRefName":"main","isDraft":false,"comments":[]},
      {"number":68,"headRefOid":"aaa","headRefName":"dwim/a","baseRefName":"main","isDraft":true,
       "comments":[{"body":"one"},{"body":"two"}]}
    ]"#;

    // When
    let prs = parse_list(json).expect("parsed");

    // Then
    assert_eq!(prs[0].number, 68);
    assert_eq!(prs[0].comments, ["one", "two"]);
    assert!(prs[0].draft);
    assert_eq!(prs[1].branch, "web/b");
    assert_eq!(prs[1].base, "main");
}

#[test]
fn a_listed_pr_missing_a_field_is_refused_naming_it() {
    let json =
        r#"[{"number":68,"headRefOid":"aaa","baseRefName":"main","isDraft":false,"comments":[]}]"#;
    let refused = parse_list(json).expect_err("no branch");
    assert!(
        refused.contains("headRefName") && refused.contains("#68"),
        "{refused}"
    );
}
