//! Tokenizer tests: CSS Syntax Level 3 token shapes. vapors-specific; sumi's
//! parser works on characters and has no tokenizer corpus to translate.

use vapors_css::token::{Token, TokenKind, tokenize};

fn kinds(src: &str) -> Vec<TokenKind> {
    tokenize(src)
        .expect("tokenizes")
        .into_iter()
        .map(|t| t.kind)
        .collect()
}

fn ident(s: &str) -> TokenKind {
    TokenKind::Ident(s.to_owned())
}

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
fn numbers_percentages_and_dimensions_carry_their_values() {
    // Given / When
    let got = kinds("1.5 -2 50% 10px +3e2em .5");

    // Then
    let number = |v: f64, int: bool| TokenKind::Number {
        value: v,
        integer: int,
    };
    assert_eq!(
        got,
        [
            number(1.5, false),
            TokenKind::Whitespace,
            number(-2.0, true),
            TokenKind::Whitespace,
            TokenKind::Percentage(50.0),
            TokenKind::Whitespace,
            TokenKind::Dimension {
                value: 10.0,
                unit: "px".to_owned()
            },
            TokenKind::Whitespace,
            TokenKind::Dimension {
                value: 300.0,
                unit: "em".to_owned()
            },
            TokenKind::Whitespace,
            number(0.5, false),
        ]
    );
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
fn unterminated_string_is_refused_naming_the_quote() {
    // Given / When
    let err = tokenize("content: \"open").unwrap_err();

    // Then
    assert_eq!(err.position.column, 10);
    assert!(err.message.contains("unterminated string"), "{err}");
}

#[test]
fn custom_property_names_and_escapes_are_idents() {
    // Given / When
    let got = kinds(r"--accent \31 23");

    // Then
    assert_eq!(
        got,
        [ident("--accent"), TokenKind::Whitespace, ident("123")]
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
fn unquoted_url_tokenizes_as_a_url() {
    // Given / When
    let got = kinds("url(x.woff2)");

    // Then
    assert_eq!(got, [TokenKind::Url("x.woff2".to_owned())]);
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
