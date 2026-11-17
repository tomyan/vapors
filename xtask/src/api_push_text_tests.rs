use super::published;

#[test]
fn plain_text_with_an_email_and_issue_talk_is_published() {
    let text = "F3: rows\n\n\tsee docs/plans/x.md; mail t@t.example; this fixes the row gap (#12 is related)";
    assert_eq!(published("the PR body", text), Ok(()));
}

#[test]
fn every_mention_is_refused_not_only_the_first_at() {
    for text in ["thanks @tomyan", "x@ @tomyan", "cc (@a-b)", "é@tomyan"] {
        let refused = published("the PR body", text).expect_err(text);
        assert!(refused.contains("mentions"), "{text}: {refused}");
    }
}

#[test]
fn a_closing_keyword_in_any_of_githubs_forms_is_refused() {
    for text in [
        "Fixes #12",
        "(fixes #1)",
        "Closes: tomyan/vapors#3",
        "resolved GH-4",
        "Fixes https://github.com/tomyan/vapors/issues/5",
        "closes https://github.com/tomyan/vapors/pull/6.",
    ] {
        let refused = published("a commit message", text).expect_err(text);
        assert!(refused.contains("closes an issue"), "{text}: {refused}");
    }
}

#[test]
fn a_control_byte_is_refused() {
    let refused = published("an author name", "t\u{1b}[2J").expect_err("refused");
    assert!(refused.contains("control character"), "{refused}");
}
