//! Why [`crate::ScenePainter::draw_tooltip`] drew no tooltip.

use std::fmt;

use vapors_dom::NodeId;
use vapors_text::AtlasError;

use crate::PlacementError;

/// Why the tooltip was not drawn. The scene under it was drawn.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub enum TooltipError {
    /// A `--tooltip-*` property on the root holds a value of the wrong
    /// kind.
    Property {
        /// The property, such as `--tooltip-background`.
        name: &'static str,
        /// Its value as written.
        value: String,
        /// What it takes.
        expected: &'static str,
    },
    /// The document has not been updated, so it has no styles or boxes.
    NotUpdated,
    /// A pointer tooltip, and no pointer position was given.
    NoPointer,
    /// A keyboard tooltip whose element has no box.
    NoBox(NodeId),
    /// The window or the tooltip's size could not be placed.
    Placement(PlacementError),
    /// A glyph of the text could not be drawn.
    Atlas(AtlasError),
}

impl fmt::Display for TooltipError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Property {
                name,
                value,
                expected,
            } => write!(
                f,
                "the tooltip was not drawn: `{name}: {value}` on the root is not {expected}; \
                 set it to {expected}, or remove it for the default"
            ),
            Self::NotUpdated => f.write_str(
                "the tooltip was not drawn: the document has not been updated; \
                 call `Document::update` before drawing",
            ),
            Self::NoPointer => f.write_str(
                "the tooltip was not drawn: it was brought up by the pointer and no pointer \
                 position was given; pass `DocumentInput::tooltip_hotspot()`",
            ),
            Self::NoBox(node) => write!(
                f,
                "the tooltip was not drawn: its element {node:?} has no box; ask \
                 `DocumentInput::tooltip` after the update, which hides it then"
            ),
            Self::Placement(error) => write!(f, "the tooltip was not drawn: {error}"),
            Self::Atlas(error) => write!(f, "the tooltip's text could not be drawn: {error:?}"),
        }
    }
}

impl std::error::Error for TooltipError {}
