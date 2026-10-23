//! One element's used style: everything layout, text and paint read,
//! or every refusal its computed style earns.

use vapors_css::cascade::ComputedStyle;
use vapors_scene::Color;

use super::box_style;
use super::honoured::check_honoured;
use super::text_style::{TextStyle, color, text_style};
use super::values::{Reader, Refusal, Styles, keyword};

/// What the element's computed style means to layout, text and paint.
#[derive(Clone, Debug)]
pub(crate) struct Used {
    pub(crate) layout: taffy::Style,
    pub(crate) text: TextStyle,
    pub(crate) background: Color,
    /// `visibility: visible`; `hidden` and `collapse` paint nothing of
    /// the element's own and do not take the pointer.
    pub(crate) visible: bool,
    /// `pointer-events: auto`.
    pub(crate) takes_pointer: bool,
}

/// The used style of an element whose computed styles (its own and its
/// parent's) are `styles`; `initial` holds every property's initial
/// value, and `flex_item` says its parent is a flex container.
pub(crate) fn used(
    styles: Styles<'_>,
    initial: &ComputedStyle,
    flex_item: bool,
) -> Result<Used, Vec<Refusal>> {
    let mut reader = Reader::new(styles);
    let result = Used {
        layout: box_style(&mut reader, flex_item),
        text: text_style(&mut reader),
        background: reader.read("background-color", color, Color::default()),
        visible: reader.read("visibility", |v| keyword(v).map(|k| k == "visible"), true),
        takes_pointer: reader.read("pointer-events", |v| keyword(v).map(|k| k == "auto"), true),
    };
    let mut refusals = check_honoured(styles, initial);
    refusals.extend(reader.refusals);
    if refusals.is_empty() {
        Ok(result)
    } else {
        Err(refusals)
    }
}
