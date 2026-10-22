//! Token consumers (`src/token/consume.rs`): numbers, strings, escapes, `url()`.

mod support;

use support::tokenizer::{ident, kinds};
use vapors_css::token::{TokenKind, tokenize};

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
fn unquoted_url_tokenizes_as_a_url() {
    // Given / When
    let got = kinds("url(x.woff2)");

    // Then
    assert_eq!(got, [TokenKind::Url("x.woff2".to_owned())]);
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
