//! Colour syntax: named, hex, the colour functions, `transparent`,
//! `currentColor` and `light-dark()`.

use super::color::Color;
use super::color_args::arguments;
use super::color_space;
use super::named_colors::NAMED_COLORS;
use crate::ast::ComponentValue;
use crate::parse::component::split_commas;
use crate::token::{Token, TokenKind};
use crate::{Error, Position, Result};

/// Parse one colour from component values.
pub(super) fn parse(values: &[ComponentValue]) -> Result<Color> {
    let at = values
        .first()
        .map_or(Position { line: 1, column: 1 }, ComponentValue::position);
    match values {
        [
            ComponentValue::Token(Token {
                kind: TokenKind::Ident(name),
                ..
            }),
        ] => named(name, at),
        [
            ComponentValue::Token(Token {
                kind: TokenKind::Hash { value, .. },
                ..
            }),
        ] => hex(value, at),
        [
            ComponentValue::Function {
                name,
                args,
                position,
            },
        ] => function(name, args, *position),
        _ => Err(Error::new(
            at,
            "expected a colour: a name, #hex, rgb(), hsl(), hwb(), lab(), lch(), oklab(), oklch() or light-dark()",
        )),
    }
}

fn named(name: &str, at: Position) -> Result<Color> {
    let lower = name.to_ascii_lowercase();
    if lower == "transparent" {
        return Ok(Color::TRANSPARENT);
    }
    if lower == "currentcolor" {
        return Ok(Color::CurrentColor);
    }
    let index = NAMED_COLORS
        .binary_search_by_key(&lower.as_str(), |(n, _)| n)
        .map_err(|_| Error::new(at, format!("{name:?} is not a named colour")))?;
    let rgb = NAMED_COLORS[index].1;
    Ok(Color::from_channels(
        f64::from((rgb >> 16) & 0xff),
        f64::from((rgb >> 8) & 0xff),
        f64::from(rgb & 0xff),
        1.0,
    ))
}

fn hex(digits: &str, at: Position) -> Result<Color> {
    let refuse = || {
        Error::new(
            at,
            format!("#{digits} is not a hex colour; use 3, 4, 6 or 8 hex digits"),
        )
    };
    if !digits.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(refuse());
    }
    let expanded: String = match digits.len() {
        3 | 4 => digits.chars().flat_map(|c| [c, c]).collect(),
        6 | 8 => digits.to_owned(),
        _ => return Err(refuse()),
    };
    let channel = |i: usize| u8::from_str_radix(&expanded[i..i + 2], 16).map(f64::from);
    let alpha = if expanded.len() == 8 {
        channel(6).map_err(|_| refuse())? / 255.0
    } else {
        1.0
    };
    Ok(Color::from_channels(
        channel(0).map_err(|_| refuse())?,
        channel(2).map_err(|_| refuse())?,
        channel(4).map_err(|_| refuse())?,
        alpha,
    ))
}

fn function(name: &str, args: &[ComponentValue], at: Position) -> Result<Color> {
    let lower = name.to_ascii_lowercase();
    if lower == "light-dark" {
        return light_dark(args, at);
    }
    let (components, alpha) = arguments(args, at)?;
    let [c0, c1, c2] = components;
    let (r, g, b) = match lower.as_str() {
        "rgb" | "rgba" => (c0.scaled(255.0), c1.scaled(255.0), c2.scaled(255.0)),
        "hsl" | "hsla" => color_space::hsl_to_rgb(c0.angle(), c1.fraction(), c2.fraction()),
        "hwb" => color_space::hwb_to_rgb(c0.angle(), c1.fraction(), c2.fraction()),
        "lab" => color_space::lab_to_rgb(c0.scaled(100.0), c1.scaled(125.0), c2.scaled(125.0)),
        "lch" => {
            let (a, b) = color_space::polar_to_ab(c1.scaled(150.0), c2.angle());
            color_space::lab_to_rgb(c0.scaled(100.0), a, b)
        }
        "oklab" => color_space::oklab_to_rgb(c0.scaled(1.0), c1.scaled(0.4), c2.scaled(0.4)),
        "oklch" => {
            let (a, b) = color_space::polar_to_ab(c1.scaled(0.4), c2.angle());
            color_space::oklab_to_rgb(c0.scaled(1.0), a, b)
        }
        other => {
            return Err(Error::new(
                at,
                format!("{other}() is not a colour function"),
            ));
        }
    };
    Ok(Color::from_channels(r, g, b, alpha))
}

fn light_dark(args: &[ComponentValue], at: Position) -> Result<Color> {
    let parts = split_commas(args);
    let [light, dark] = parts.as_slice() else {
        return Err(Error::new(at, "light-dark() takes two colours"));
    };
    Ok(Color::LightDark(
        Box::new(parse(light)?),
        Box::new(parse(dark)?),
    ))
}
