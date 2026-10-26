//! Each rule is set, or excused with its reason; neither fails.

use super::*;

#[test]
fn an_excused_rule_says_why_and_one_neither_set_nor_excused_fails() {
    // Given: claude and plan excused, patterns neither set nor excused
    let config = Config::parse(
        "[docs.not_applying]\nclaude = \"no CLAUDE.md here\"\nplan = \"planned elsewhere\"\n",
    )
    .expect("valid");

    // When
    let outcome = rules(&config, true);

    // Then
    assert_eq!(
        outcome,
        Err(vec!["[docs] patterns is not set and [docs.not_applying] gives no reason; fix: set one in .config/factory.toml".to_owned()])
    );
}

#[test]
fn every_rule_excused_passes_saying_each_reason() {
    let config =
        Config::parse("[docs.not_applying]\nclaude = \"a\"\nplan = \"b\"\npatterns = \"c\"\n")
            .expect("valid");
    assert_eq!(
        rules(&config, true),
        Ok(vec![
            "claude: does not apply here: a".to_owned(),
            "plan: does not apply here: b".to_owned(),
            "patterns: does not apply here: c".to_owned(),
        ])
    );
}
