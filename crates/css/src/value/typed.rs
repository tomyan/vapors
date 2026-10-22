//! The typed value a declaration parses to, and its canonical CSS text.

// r, g, b, a are the channel names the colour syntax uses.
#![allow(clippy::many_single_char_names)]

use std::fmt::Write;

use super::{Calc, Color, Length, LengthUnit, typed_calc};

/// A parsed property value. Shapes follow the property's syntax: a
/// keyword, one typed value, a space-separated `List`, a `CommaList`, or
/// a function with its arguments.
#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    /// An identifier, lowercased.
    Keyword(String),
    /// A length or percentage.
    Length(Length),
    /// A plain number.
    Number(f64),
    /// An integer.
    Integer(i64),
    /// A percentage where the syntax wants one on its own (opacity).
    Percentage(f64),
    /// A colour.
    Color(Color),
    /// A quoted string.
    String(String),
    /// A duration.
    Time(Time),
    /// A `fr` track size.
    Flex(f64),
    /// `width / height`.
    Ratio(f64, f64),
    /// A math expression kept for evaluation.
    Calc(Calc),
    /// `name(args)`; arguments as the function's syntax lists them.
    Function(String, Vec<Value>),
    /// Space-separated values.
    List(Vec<Value>),
    /// Comma-separated values.
    CommaList(Vec<Value>),
}

/// A duration with the unit it was written in.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Time {
    /// The number before the unit.
    pub value: f64,
    /// `ms` or `s`.
    pub unit: TimeUnit,
}

/// Time units.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TimeUnit {
    /// Milliseconds.
    Ms,
    /// Seconds.
    S,
}

impl Time {
    /// The duration in milliseconds.
    #[must_use]
    pub fn millis(self) -> f64 {
        match self.unit {
            TimeUnit::Ms => self.value,
            TimeUnit::S => self.value * 1000.0,
        }
    }
}

impl Value {
    /// The value as canonical CSS text: colours as `rgb()`, keywords
    /// lowercased, lists joined by a space or `, `.
    #[must_use]
    pub fn to_css(&self) -> String {
        let mut out = String::new();
        self.write(&mut out);
        out
    }

    /// The keyword, if this is one.
    #[must_use]
    pub fn as_keyword(&self) -> Option<&str> {
        match self {
            Self::Keyword(k) => Some(k),
            _ => None,
        }
    }

    fn write(&self, out: &mut String) {
        match self {
            Self::Keyword(k) => out.push_str(k),
            Self::Length(l) => length(*l, out),
            Self::Number(n) | Self::Flex(n) | Self::Percentage(n) => {
                number(*n, out);
                match self {
                    Self::Flex(_) => out.push_str("fr"),
                    Self::Percentage(_) => out.push('%'),
                    _ => {}
                }
            }
            Self::Integer(i) => {
                let _ = write!(out, "{i}");
            }
            Self::Color(c) => color(c, out),
            Self::String(s) => {
                crate::serialize::token(&crate::token::TokenKind::String(s.clone()), out);
            }
            Self::Time(t) => {
                number(t.value, out);
                out.push_str(match t.unit {
                    TimeUnit::Ms => "ms",
                    TimeUnit::S => "s",
                });
            }
            Self::Ratio(w, h) => {
                number(*w, out);
                out.push_str(" / ");
                number(*h, out);
            }
            Self::Calc(c) => typed_calc::write(c, out),
            Self::Function(name, args) => {
                out.push_str(name);
                out.push('(');
                join(args, ", ", out);
                out.push(')');
            }
            Self::List(items) => join(items, " ", out),
            Self::CommaList(items) => join(items, ", ", out),
        }
    }
}

fn join(items: &[Value], sep: &str, out: &mut String) {
    for (i, item) in items.iter().enumerate() {
        if i > 0 {
            out.push_str(sep);
        }
        item.write(out);
    }
}

pub(super) fn number(n: f64, out: &mut String) {
    let _ = write!(out, "{n}");
}

/// A length as text, with its unit.
pub fn length(l: Length, out: &mut String) {
    number(l.value, out);
    out.push_str(match l.unit {
        LengthUnit::Px => "px",
        LengthUnit::Em => "em",
        LengthUnit::Rem => "rem",
        LengthUnit::Vw => "vw",
        LengthUnit::Vh => "vh",
        LengthUnit::Ch => "ch",
        LengthUnit::Percent => "%",
    });
}

/// A colour as CSS Color Level 4 serializes it.
fn color(c: &Color, out: &mut String) {
    match c {
        Color::CurrentColor => out.push_str("currentcolor"),
        Color::LightDark(light, dark) => {
            out.push_str("light-dark(");
            color(light, out);
            out.push_str(", ");
            color(dark, out);
            out.push(')');
        }
        Color::Rgba(rgba) => {
            let [r, g, b, a] = rgba.to_rgba8();
            if a == 0 && r == 0 && g == 0 && b == 0 {
                out.push_str("transparent");
            } else if a == 255 {
                let _ = write!(out, "rgb({r} {g} {b})");
            } else {
                let _ = write!(out, "rgb({r} {g} {b} / {})", f64::from(a) / 255.0);
            }
        }
    }
}
