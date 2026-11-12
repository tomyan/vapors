//! An inline `<svg>` icon (docs/icons.md): the numbers its attributes
//! hold, its natural size (the `width` and `height` attributes in CSS
//! px), and the Taffy style those give it. What the subset refuses is `svg_checks` and
//! `svg_path_checks`; layout reads an svg only once they pass.

use taffy::Style;
use vapors_dom::NodeId;

use crate::arena::Arena;

/// The SVG namespace, the one `xmlns` value the subset takes.
pub(crate) const NAMESPACE: &str = "http://www.w3.org/2000/svg";

/// Whether `node` is an element with tag `tag`.
pub(crate) fn is(arena: &Arena, node: NodeId, tag: &str) -> bool {
    arena
        .get(node, "svg")
        .element()
        .is_some_and(|element| element.tag == tag)
}

/// `text` as a positive, finite number with no unit; `None` otherwise.
pub(crate) fn positive(text: &str) -> Option<f32> {
    number(text).filter(|n| *n > 0.0)
}

/// `token` as a CSS `<number>`, finite as an f32; `None` otherwise.
/// `f32::from_str` also takes `16.`, `1.e1` and `inf`, which Chromium
/// drops from a `width` or `viewBox`, so the shape is checked first.
fn number(token: &str) -> Option<f32> {
    if !is_css_number(token) {
        return None;
    }
    token.parse::<f32>().ok().filter(|n| n.is_finite())
}

/// Whether `token` is `[+-]? (digits | digits? '.' digits)
/// ([eE] [+-]? digits)?`.
fn is_css_number(token: &str) -> bool {
    let unsigned = token.strip_prefix(['+', '-']).unwrap_or(token);
    let (mantissa, exponent) = match unsigned.split_once(['e', 'E']) {
        Some((mantissa, exponent)) => (mantissa, Some(exponent)),
        None => (unsigned, None),
    };
    let mantissa_ok = match mantissa.split_once('.') {
        Some((whole, fraction)) => (whole.is_empty() || digits(whole)) && digits(fraction),
        None => digits(mantissa),
    };
    mantissa_ok && exponent.is_none_or(|e| digits(e.strip_prefix(['+', '-']).unwrap_or(e)))
}

fn digits(text: &str) -> bool {
    !text.is_empty() && text.bytes().all(|b| b.is_ascii_digit())
}

/// A `viewBox`'s numbers, separated by whitespace and at most one comma
/// each; `None` unless there are four finite ones.
pub(crate) fn view_box(text: &str) -> Option<[f32; 4]> {
    let mut numbers = Vec::new();
    for part in text.split(',') {
        let before = numbers.len();
        for token in part.split_ascii_whitespace() {
            numbers.push(number(token)?);
        }
        if numbers.len() == before {
            return None;
        }
    }
    numbers.try_into().ok()
}

/// The natural size of `node` when it is an `svg`: its `width` and
/// `height` attributes, CSS px; `None` for any other element, or an svg
/// whose sizes `svg_checks` refuses.
pub(crate) fn natural(arena: &Arena, node: NodeId) -> Option<[f32; 2]> {
    let element = arena.get(node, "svg").element()?;
    if element.tag != "svg" {
        return None;
    }
    let size = |name| positive(element.attributes.get(name)?);
    Some([size("width")?, size("height")?])
}

/// The Taffy style of `node` styled `style`. An svg's `width` and
/// `height` attributes are its CSS width and height, as Chromium maps
/// them (presentational hints), so they follow `box-sizing` and a flex
/// item keeps them unless it grows, shrinks or has a basis. Chromium
/// gives an svg flex item no automatic minimum size, so its min size is
/// 0 (`tests/svg_size.rs`). CSS sizes on an svg are refused
/// (`svg_style_checks`), so none is overwritten.
pub(crate) fn layout_style(arena: &Arena, node: NodeId, style: &Style) -> Style {
    let mut style = style.clone();
    if let Some([width, height]) = natural(arena, node) {
        style.size = taffy::Size::from_lengths(width, height);
        style.min_size = taffy::Size::zero();
    }
    style
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_view_box_takes_whitespace_and_single_commas() {
        for (text, expected) in [
            ("0 0 16 16", Some([0.0, 0.0, 16.0, 16.0])),
            ("0,0,16,16", Some([0.0, 0.0, 16.0, 16.0])),
            (" -1 , 2 3.5\n4 ", Some([-1.0, 2.0, 3.5, 4.0])),
            ("0 0,,16 16", None),
            (",0 0 16 16", None),
            ("0 0 16", None),
            ("0 0 16 16 16", None),
            ("0 0 16 inf", None),
            ("0 0 16px 16", None),
            ("0 0 +8 16", Some([0.0, 0.0, 8.0, 16.0])),
            ("0 0 .8e1 16", Some([0.0, 0.0, 8.0, 16.0])),
            ("0 0 8. 16", None),
            ("0 0 8.e0 16", None),
            ("0 0 8e 16", None),
        ] {
            // When / Then
            assert_eq!(view_box(text), expected, "{text:?}");
        }
    }

    #[test]
    /// Each case checked against Chromium 155, which drops a `width`
    /// or `viewBox` outside the CSS `<number>` grammar (`16.`, `1.e1`).
    fn a_size_is_a_positive_number_with_no_unit() {
        for (text, expected) in [
            ("16", Some(16.0)),
            ("1.5", Some(1.5)),
            ("1e1", Some(10.0)),
            ("16px", None),
            (" 16", None),
            ("0", None),
            ("-2", None),
            ("inf", None),
            ("NaN", None),
            ("+16", Some(16.0)),
            (".5", Some(0.5)),
            ("1E1", Some(10.0)),
            ("1e+1", Some(10.0)),
            ("1e-1", Some(0.1)),
            ("16e0", Some(16.0)),
            ("16.", None),
            ("1.e1", None),
            ("1e", None),
            ("e1", None),
            (".", None),
            ("+", None),
            ("1e+", None),
            ("1..5", None),
            ("infinity", None),
            ("0x10", None),
            ("1_6", None),
            ("1e40", None),
        ] {
            // When / Then
            assert_eq!(positive(text), expected, "{text:?}");
        }
    }
}
