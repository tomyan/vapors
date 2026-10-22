//! Helpers that were in `tests/tokenizer.rs`.
#![allow(dead_code)]

use vapors_css::token::{TokenKind, tokenize};

pub fn kinds(src: &str) -> Vec<TokenKind> {
    tokenize(src)
        .expect("tokenizes")
        .into_iter()
        .map(|t| t.kind)
        .collect()
}

pub fn ident(s: &str) -> TokenKind {
    TokenKind::Ident(s.to_owned())
}
