//! The pointer's shape over the window, as the app asks for it: the CSS
//! cursor keywords, each as winit's cursor icon. `None` hides it.

use winit::window::{CursorIcon, Window};

/// A cursor shape; each variant is the CSS keyword of the same name.
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

/// winit's icon for `cursor`; `None` for the hidden cursor.
fn icon(cursor: Cursor) -> Option<CursorIcon> {
    Some(match cursor {
        Cursor::None => return None,
        Cursor::Default => CursorIcon::Default,
        Cursor::Pointer => CursorIcon::Pointer,
        Cursor::Text => CursorIcon::Text,
        Cursor::Move => CursorIcon::Move,
        Cursor::Grab => CursorIcon::Grab,
        Cursor::Grabbing => CursorIcon::Grabbing,
        Cursor::NotAllowed => CursorIcon::NotAllowed,
        Cursor::Crosshair => CursorIcon::Crosshair,
        Cursor::ColResize => CursorIcon::ColResize,
        Cursor::RowResize => CursorIcon::RowResize,
        Cursor::EwResize => CursorIcon::EwResize,
        Cursor::NsResize => CursorIcon::NsResize,
        Cursor::NeswResize => CursorIcon::NeswResize,
        Cursor::NwseResize => CursorIcon::NwseResize,
        Cursor::Wait => CursorIcon::Wait,
        Cursor::Progress => CursorIcon::Progress,
        Cursor::Help => CursorIcon::Help,
        Cursor::ZoomIn => CursorIcon::ZoomIn,
        Cursor::ZoomOut => CursorIcon::ZoomOut,
    })
}

/// Shows `cursor` over `window`.
pub(crate) fn apply(window: &Window, cursor: Cursor) {
    match icon(cursor) {
        Some(icon) => {
            window.set_cursor_visible(true);
            window.set_cursor(icon);
        }
        None => window.set_cursor_visible(false),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn none_hides_the_cursor_and_the_rest_have_icons() {
        // Given
        let (hidden, pointer) = (icon(Cursor::None), icon(Cursor::Pointer));

        // Then
        assert_eq!(hidden, None);
        assert_eq!(pointer, Some(CursorIcon::Pointer));
    }
}
