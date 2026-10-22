//! Tokenizer (`src/token/mod.rs`): token shapes per CSS Syntax Level 3. vapors-specific; sumi tokenizes by character.

mod support;

use support::tokenizer::{ident, kinds};
use vapors_css::token::{Token, TokenKind, tokenize};

#[test]
fn ident_colon_and_semicolon_tokenize_in_order() {
    // Given
    let src = "color:red;";

    // When
    let got = kinds(src);

    // Then
    assert_eq!(
        got,
        [
            ident("color"),
            TokenKind::Colon,
            ident("red"),
            TokenKind::Semicolon
        ]
    );
}

#[test]
fn whitespace_runs_collapse_to_one_token() {
    // Given / When
    let got = kinds("a \t\n b");

    // Then
    assert_eq!(got, [ident("a"), TokenKind::Whitespace, ident("b")]);
}

#[test]
fn comments_are_dropped() {
    // Given / When
    let got = kinds("a/* x */b");

    // Then
    assert_eq!(got, [ident("a"), ident("b")]);
}

#[test]
fn unterminated_comment_is_refused_at_its_start() {
    // Given / When
    let err = tokenize("a\n  /* open").unwrap_err();

    // Then
    assert_eq!((err.position.line, err.position.column), (2, 3));
    assert!(err.message.contains("unterminated comment"), "{err}");
}

#[test]
fn functions_hashes_at_keywords_and_strings() {
    // Given / When
    let got = kinds(r#"rgb(#fff @media "a" 'b')"#);

    // Then
    assert_eq!(
        got,
        [
            TokenKind::Function("rgb".to_owned()),
            TokenKind::Hash {
                value: "fff".to_owned(),
                is_id: true
            },
            TokenKind::Whitespace,
            TokenKind::AtKeyword("media".to_owned()),
            TokenKind::Whitespace,
            TokenKind::String("a".to_owned()),
            TokenKind::Whitespace,
            TokenKind::String("b".to_owned()),
            TokenKind::CloseParen,
        ]
    );
}

#[test]
fn hash_starting_with_a_digit_is_not_an_id() {
    // Given / When
    let got = kinds("#1a2");

    // Then
    assert_eq!(
        got,
        [TokenKind::Hash {
            value: "1a2".to_owned(),
            is_id: false
        }]
    );
}

#[test]
fn delimiters_and_brackets() {
    // Given / When
    let got = kinds("a>b[c]{d},*");

    // Then
    assert_eq!(
        got,
        [
            ident("a"),
            TokenKind::Delim('>'),
            ident("b"),
            TokenKind::OpenSquare,
            ident("c"),
            TokenKind::CloseSquare,
            TokenKind::OpenCurly,
            ident("d"),
            TokenKind::CloseCurly,
            TokenKind::Comma,
            TokenKind::Delim('*'),
        ]
    );
}

#[test]
fn every_token_records_its_line_and_column() {
    // Given / When
    let tokens: Vec<Token> = tokenize("a {\n  b: c;\n}").expect("tokenizes");

    // Then
    let b = tokens
        .iter()
        .find(|t| t.kind == ident("b"))
        .expect("b present");
    assert_eq!((b.position.line, b.position.column), (2, 3));
    let close = tokens.last().expect("close curly");
    assert_eq!((close.position.line, close.position.column), (3, 1));
}

#[test]
fn cdo_and_cdc_are_dropped() {
    // Given / When
    let got = kinds("<!-- a -->");

    // Then
    assert_eq!(
        got,
        [TokenKind::Whitespace, ident("a"), TokenKind::Whitespace]
    );
}
