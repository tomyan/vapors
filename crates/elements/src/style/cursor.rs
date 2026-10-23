//! The `cursor` property: the pointer's shape over an element. `auto`
//! is the default arrow until text selection arrives (F4), where a
//! browser shows the text cursor over text.

use vapors_css::value::Value;

use super::values::{keyword, unknown};

/// A cursor shape, one per keyword of the CSS grammar except `auto`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[allow(missing_docs)] // each variant is the CSS keyword of the same name
pub enum Cursor {
    Default,
    None,
    Pointer,
    Text,
    Move,
    Grab,
    Grabbing,
    NotAllowed,
    Crosshair,
    ColResize,
    RowResize,
    EwResize,
    NsResize,
    NeswResize,
    NwseResize,
    Wait,
    Progress,
    Help,
    ZoomIn,
    ZoomOut,
}

/// Every cursor, for a mapping that must cover them all.
pub const ALL_CURSORS: [Cursor; 20] = [
    Cursor::Default,
    Cursor::None,
    Cursor::Pointer,
    Cursor::Text,
    Cursor::Move,
    Cursor::Grab,
    Cursor::Grabbing,
    Cursor::NotAllowed,
    Cursor::Crosshair,
    Cursor::ColResize,
    Cursor::RowResize,
    Cursor::EwResize,
    Cursor::NsResize,
    Cursor::NeswResize,
    Cursor::NwseResize,
    Cursor::Wait,
    Cursor::Progress,
    Cursor::Help,
    Cursor::ZoomIn,
    Cursor::ZoomOut,
];

pub(crate) fn cursor(value: &Value) -> Result<Cursor, String> {
    Ok(match keyword(value)? {
        "auto" | "default" => Cursor::Default,
        "none" => Cursor::None,
        "pointer" => Cursor::Pointer,
        "text" => Cursor::Text,
        "move" => Cursor::Move,
        "grab" => Cursor::Grab,
        "grabbing" => Cursor::Grabbing,
        "not-allowed" => Cursor::NotAllowed,
        "crosshair" => Cursor::Crosshair,
        "col-resize" => Cursor::ColResize,
        "row-resize" => Cursor::RowResize,
        "ew-resize" => Cursor::EwResize,
        "ns-resize" => Cursor::NsResize,
        "nesw-resize" => Cursor::NeswResize,
        "nwse-resize" => Cursor::NwseResize,
        "wait" => Cursor::Wait,
        "progress" => Cursor::Progress,
        "help" => Cursor::Help,
        "zoom-in" => Cursor::ZoomIn,
        "zoom-out" => Cursor::ZoomOut,
        other => return Err(unknown(other)),
    })
}
