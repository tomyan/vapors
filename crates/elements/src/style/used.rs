//! One element's used style: everything layout, text and paint read,
//! or every refusal its computed style earns.

use vapors_css::cascade::ComputedStyle;
use vapors_scene::Color;

use super::box_style;
use super::honoured::check_honoured;
use super::text_style::{TextStyle, color, text_style};
use super::values::{Reader, Refusal, Styles};

/// What the element's computed style means to layout, text and paint.
#[derive(Clone, Debug)]
pub(crate) struct Used {
    pub(crate) layout: taffy::Style,
    pub(crate) text: TextStyle,
    pub(crate) background: Color,
}

/// The used style of an element whose computed styles (its own and its
/// parent's) are `styles`;
/// `initial` holds every property's initial value.
pub(crate) fn used(styles: Styles<'_>, initial: &ComputedStyle) -> Result<Used, Vec<Refusal>> {
    let mut reader = Reader::new(styles);
    let result = Used {
        layout: box_style(&mut reader),
        text: text_style(&mut reader),
        background: reader.read("background-color", color, Color::default()),
    };
    let mut refusals = check_honoured(styles, initial);
    refusals.extend(reader.refusals);
    if refusals.is_empty() {
        Ok(result)
    } else {
        Err(refusals)
    }
}
