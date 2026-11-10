//! An inline `<svg>` icon (docs/icons.md): the numbers its attributes
//! hold, and its natural size, the `width` and `height` attributes in
//! CSS px. What the subset refuses is `svg_checks` and
//! `svg_path_checks`; layout reads an svg only once they pass.

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
    text.parse::<f32>()
        .ok()
        .filter(|n| n.is_finite() && *n > 0.0)
}

/// A `viewBox`'s numbers, separated by whitespace and at most one comma
/// each; `None` unless there are four finite ones.
pub(crate) fn view_box(text: &str) -> Option<[f32; 4]> {
    let mut numbers = Vec::new();
    for part in text.split(',') {
        let before = numbers.len();
        for token in part.split_ascii_whitespace() {
            numbers.push(token.parse::<f32>().ok().filter(|n| n.is_finite())?);
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
        ] {
            // When / Then
            assert_eq!(view_box(text), expected, "{text:?}");
        }
    }

    #[test]
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
        ] {
            // When / Then
            assert_eq!(positive(text), expected, "{text:?}");
        }
    }
}
