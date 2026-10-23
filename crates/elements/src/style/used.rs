//! One element's used style: everything layout, text and paint read,
//! or every refusal its computed style earns.

use vapors_css::cascade::ComputedStyle;
use vapors_scene::Color;

use super::box_style;
use super::cursor::{Cursor, cursor};
use super::decoration::{Decoration, decoration};
use super::honoured::check_honoured;
use super::inline;
use super::text_style::{TextStyle, color, text_style};
use super::values::{Reader, Refusal, Styles, keyword};

/// What the element's computed style means to layout, text and paint.
#[derive(Clone, Debug)]
pub(crate) struct Used {
    pub(crate) layout: taffy::Style,
    pub(crate) text: TextStyle,
    pub(crate) background: Color,
    pub(crate) decoration: Decoration,
    pub(crate) cursor: Cursor,
    /// `visibility: visible`; `hidden` and `collapse` paint nothing of
    /// the element's own and do not take the pointer.
    pub(crate) visible: bool,
    /// `pointer-events: auto`.
    pub(crate) takes_pointer: bool,
    /// `display: inline`: its text joins its parent's inline run.
    pub(crate) inline: bool,
}

/// What a used style depends on besides the element's computed styles.
pub(crate) struct Context<'a> {
    /// Every property at its initial value.
    pub(crate) initial: &'a ComputedStyle,
    /// The parent is a flex or grid container, which blockifies it.
    pub(crate) flex_item: bool,
    /// Device pixels per CSS pixel, for border widths.
    pub(crate) scale: f32,
}

/// The used style of an element whose computed styles (its own and its
/// parent's) are `styles`.
pub(crate) fn used(styles: Styles<'_>, context: &Context<'_>) -> Result<Used, Vec<Refusal>> {
    let mut reader = Reader::new(styles);
    let text = text_style(&mut reader);
    let decoration = decoration(&mut reader, context.scale, text.color);
    let layout = box_style(&mut reader, context.flex_item, decoration.border);
    let inline = inline::is_inline(&mut reader, context.flex_item);
    if inline {
        inline::refuse_box_edges(&mut reader, &layout);
    }
    let result = Used {
        layout,
        inline,
        text,
        background: reader.read("background-color", color, Color::default()),
        decoration,
        cursor: reader.read("cursor", cursor, Cursor::Default),
        visible: reader.read("visibility", |v| keyword(v).map(|k| k == "visible"), true),
        takes_pointer: reader.read("pointer-events", |v| keyword(v).map(|k| k == "auto"), true),
    };
    let mut refusals = check_honoured(styles, context.initial);
    refusals.extend(reader.refusals);
    if refusals.is_empty() {
        Ok(result)
    } else {
        Err(refusals)
    }
}
