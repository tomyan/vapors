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
    /// `z-index` where it applies (a positioned box, or a flex or grid
    /// item), `None` for auto and where it does not.
    pub(crate) z_index: Option<i32>,
    /// How its thumb is drawn, when it scrolls.
    pub(crate) scrollbar: super::Scrollbar,
    /// How its track and thumb are drawn, when it is a range (3-E3a).
    pub(crate) parts: super::Parts,
}

/// What a used style depends on besides the element's computed styles.
#[allow(clippy::struct_excessive_bools)] // four independent facts about the element's place
pub(crate) struct Context<'a> {
    /// Every property at its initial value.
    pub(crate) initial: &'a ComputedStyle,
    /// The parent is a flex or grid container, which blockifies it.
    pub(crate) flex_item: bool,
    /// The parent is an inline element.
    pub(crate) in_inline: bool,
    /// The element's `overflow` belongs to the viewport
    /// (`overflow::to_viewport`).
    pub(crate) to_viewport: bool,
    /// Device pixels per CSS pixel, for border widths.
    pub(crate) scale: f32,
    /// The element is an `input type=number`, which needs a width of
    /// its own (3-E3c).
    pub(crate) number_field: bool,
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
    } else if context.in_inline && layout.display != taffy::Display::None {
        reader.refuse("display", inline::BLOCK_IN_INLINE.to_owned());
    }
    // pushed directly: `Reader::refuse` leaves an undeclared value to
    // its declarer, and `auto` here is nobody's declaration
    if context.number_field && layout.size.width == taffy::Dimension::auto() {
        reader.refusals.push(Refusal {
            property: "width",
            value: "auto".to_owned(),
            reason: "is `auto` on a number field; vapors does not model Chromium's default width (its font and spin button), so give it a width, such as `width: 6em`".to_owned(),
            declared: None,
        });
    }
    super::overflow::refuse_rounded_clip(&mut reader, &layout, &decoration);
    super::overflow::refuse_viewport_scroll(&mut reader, &layout, context.to_viewport);
    let mut result = Used {
        parts: super::parts::DEFAULT,
        z_index: applied_z_index(&mut reader, &layout, context.flex_item),
        scrollbar: super::scrollbar::scrollbar(&mut reader),
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
    let parts = super::parts::parts(styles, &mut refusals);
    result.parts = parts;
    if refusals.is_empty() {
        Ok(result)
    } else {
        Err(refusals)
    }
}

impl Used {
    /// `overflow` is `hidden` or `clip`: descendants are cut off at the
    /// padding box.
    pub(crate) fn clips(&self) -> bool {
        self.layout.overflow.x != taffy::Overflow::Visible
    }

    /// `overflow` is `scroll` or `auto`: a scroll container.
    pub(crate) fn scrolls(&self) -> bool {
        self.layout.overflow.x == taffy::Overflow::Scroll
    }
}

/// CSS 2 9.9.1 gives `z-index` to positioned boxes; CSS Flexbox 4.3
/// and Grid 9 give it to their items too, positioned or not.
fn applied_z_index(reader: &mut Reader<'_>, layout: &taffy::Style, item: bool) -> Option<i32> {
    let z = reader.read("z-index", z_index, None);
    z.filter(|_| item || layout.position != taffy::Position::Static)
}

fn z_index(value: &vapors_css::value::Value) -> Result<Option<i32>, String> {
    match value {
        vapors_css::value::Value::Keyword(k) if k == "auto" => Ok(None),
        vapors_css::value::Value::Integer(n) => i32::try_from(*n)
            .map(Some)
            .map_err(|_| "is outside the range of a 32-bit integer".to_owned()),
        _ => Err(super::values::shape("`auto` or an integer")),
    }
}
