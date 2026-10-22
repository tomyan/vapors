//! The character cursor (`src/token/cursor.rs`): input preprocessing.

mod support;

use support::tokenizer::ident;
use vapors_css::token::tokenize;

#[test]
fn input_is_preprocessed_per_syntax_level_3() {
    // Given: review item 11; CR, CRLF and FF become LF, NUL becomes U+FFFD.
    let tokens = tokenize("a\r\nb\rc\u{c}d \0").expect("tokenizes");
    let d = tokens.iter().find(|t| t.kind == ident("d")).expect("d");
    assert_eq!((d.position.line, d.position.column), (4, 1));
    assert_eq!(tokens.last().map(|t| &t.kind), Some(&ident("\u{fffd}")));
}
