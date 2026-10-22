//! Component values and stylesheets back to CSS text (CSS Syntax Level 3
//! section 6). Used for error messages, `var()` diagnostics and the
//! round-trip the parser tests check.

use std::fmt::Write;

use crate::ast::{ComponentValue, Condition, Declaration, Keyframes, Rule, Stylesheet};
use crate::token::TokenKind;

/// Component values as CSS text.
#[must_use]
pub fn component_values(values: &[ComponentValue]) -> String {
    let mut out = String::new();
    for v in values {
        component_value(v, &mut out);
    }
    out
}

fn component_value(value: &ComponentValue, out: &mut String) {
    match value {
        ComponentValue::Token(t) => token(&t.kind, out),
        ComponentValue::Function { name, args, .. } => {
            out.push_str(name);
            out.push('(');
            out.push_str(&component_values(args));
            out.push(')');
        }
        ComponentValue::Block { open, contents, .. } => {
            let (l, r) = match open {
                TokenKind::OpenSquare => ('[', ']'),
                TokenKind::OpenCurly => ('{', '}'),
                _ => ('(', ')'),
            };
            out.push(l);
            out.push_str(&component_values(contents));
            out.push(r);
        }
    }
}

/// One token as CSS text.
pub fn token(kind: &TokenKind, out: &mut String) {
    match kind {
        TokenKind::Ident(s) => ident(s, out),
        TokenKind::Function(s) => {
            ident(s, out);
            out.push('(');
        }
        TokenKind::AtKeyword(s) => {
            out.push('@');
            ident(s, out);
        }
        TokenKind::Hash { value, .. } => {
            out.push('#');
            out.push_str(value);
        }
        TokenKind::String(s) => string(s, out),
        TokenKind::Url(s) => {
            let _ = write!(out, "url({s})");
        }
        TokenKind::Number { value, .. } => number(*value, out),
        TokenKind::Percentage(v) => {
            number(*v, out);
            out.push('%');
        }
        TokenKind::Dimension { value, unit } => {
            number(*value, out);
            out.push_str(unit);
        }
        TokenKind::Whitespace => out.push(' '),
        TokenKind::Colon => out.push(':'),
        TokenKind::Semicolon => out.push(';'),
        TokenKind::Comma => out.push(','),
        TokenKind::OpenSquare => out.push('['),
        TokenKind::CloseSquare => out.push(']'),
        TokenKind::OpenParen => out.push('('),
        TokenKind::CloseParen => out.push(')'),
        TokenKind::OpenCurly => out.push('{'),
        TokenKind::CloseCurly => out.push('}'),
        TokenKind::Delim(c) => out.push(*c),
    }
}

fn number(value: f64, out: &mut String) {
    let _ = write!(out, "{value}");
}

/// An identifier with the characters that need escaping escaped.
fn ident(name: &str, out: &mut String) {
    for (i, c) in name.chars().enumerate() {
        let plain = c.is_ascii_alphanumeric() || c == '-' || c == '_' || !c.is_ascii();
        if plain && !(i == 0 && c.is_ascii_digit()) {
            out.push(c);
        } else {
            let _ = write!(out, "\\{:x} ", c as u32);
        }
    }
}

fn string(text: &str, out: &mut String) {
    out.push('"');
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\a "),
            c => out.push(c),
        }
    }
    out.push('"');
}

/// A whole stylesheet as CSS text.
#[must_use]
pub fn stylesheet(sheet: &Stylesheet) -> String {
    let mut out = String::new();
    for rule in &sheet.rules {
        rule_text(rule, &mut out);
    }
    for kf in &sheet.keyframes {
        keyframes(kf, &mut out);
    }
    out
}

fn rule_text(rule: &Rule, out: &mut String) {
    for c in &rule.conditions {
        let (name, body) = match c {
            Condition::Media(v) => ("@media", v),
            Condition::Container(v) => ("@container", v),
            Condition::Supports(v) => ("@supports", v),
        };
        let _ = write!(out, "{name} {} {{ ", component_values(body));
    }
    out.push_str(&component_values(&rule.prelude));
    out.push_str(" { ");
    declarations(&rule.declarations, out);
    out.push('}');
    for _ in &rule.conditions {
        out.push_str(" }");
    }
    out.push('\n');
}

fn declarations(decls: &[Declaration], out: &mut String) {
    for d in decls {
        let _ = write!(out, "{}: {}", d.name, component_values(&d.value));
        if d.important {
            out.push_str(" !important");
        }
        out.push_str("; ");
    }
}

fn keyframes(kf: &Keyframes, out: &mut String) {
    let _ = write!(out, "@keyframes {} {{ ", kf.name);
    for stop in &kf.stops {
        let _ = write!(out, "{}% {{ ", stop.offset * 100.0);
        declarations(&stop.declarations, out);
        out.push_str("} ");
    }
    out.push_str("}\n");
}
