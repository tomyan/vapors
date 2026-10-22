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
    let number = |v: f64, int: bool, signed: bool| TokenKind::Number {
        value: v,
        integer: int,
        signed,
    };
    assert_eq!(
        got,
        [
            number(1.5, false, false),
            TokenKind::Whitespace,
            number(-2.0, true, true),
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
            number(0.5, false, false),
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

#[test]
fn input_is_preprocessed_per_syntax_level_3() {
    // Given: review item 11; CR, CRLF and FF become LF, NUL becomes U+FFFD.
    let tokens = tokenize("a\r\nb\rc\u{c}d \0").expect("tokenizes");
    let d = tokens.iter().find(|t| t.kind == ident("d")).expect("d");
    assert_eq!((d.position.line, d.position.column), (4, 1));
    assert_eq!(tokens.last().map(|t| &t.kind), Some(&ident("\u{fffd}")));
}

#[test]
fn escapes_consume_at_most_six_hex_digits_and_null_becomes_replacement() {
    assert_eq!(kinds(r"\0000411"), [ident("A1")]);
    assert_eq!(kinds(r"\0 x"), [ident("\u{fffd}x")]);
    assert_eq!(kinds(r"\110000 x"), [ident("\u{fffd}x")], "out of range");
}

#[test]
fn unquoted_url_resolves_escapes() {
    assert_eq!(kinds(r"url(a\)b)"), [TokenKind::Url("a)b".to_owned())]);
    assert_eq!(kinds(r"url(a\41 b)"), [TokenKind::Url("aAb".to_owned())]);
}

#[test]
fn a_refusal_at_the_end_of_input_points_past_the_last_token() {
    // Given: review item 17; the brace is missing at the end of input.
    let err = vapors_css::parse(".ab\n").unwrap_err();
    assert_eq!((err.position.line, err.position.column), (2, 1));
}
