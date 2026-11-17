use super::{LARGEST, tool_prompt_tokens};

#[test]
fn a_model_with_both_columns_published_takes_the_larger() {
    assert_eq!(tool_prompt_tokens("claude-sonnet-5"), 474);
    assert_eq!(tool_prompt_tokens("claude-haiku-5-5"), 406);
}

#[test]
fn a_blank_column_or_an_unknown_model_takes_the_largest_published_figure() {
    assert_eq!(tool_prompt_tokens("claude-sonnet-5-5"), LARGEST);
    assert_eq!(tool_prompt_tokens("claude-opus-5-5"), LARGEST);
    assert_eq!(tool_prompt_tokens("claude-new-6"), LARGEST);
}
