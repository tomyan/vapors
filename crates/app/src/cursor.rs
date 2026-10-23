//! The document's cursor as the platform's: one variant each, by the
//! CSS keyword they share.

use vapors_elements::Cursor;
use vapors_platform::Cursor as Shown;

/// The platform cursor for the document's.
pub(crate) fn shown(cursor: Cursor) -> Shown {
    match cursor {
        Cursor::Default => Shown::Default,
        Cursor::None => Shown::None,
        Cursor::Pointer => Shown::Pointer,
        Cursor::Text => Shown::Text,
        Cursor::Move => Shown::Move,
        Cursor::Grab => Shown::Grab,
        Cursor::Grabbing => Shown::Grabbing,
        Cursor::NotAllowed => Shown::NotAllowed,
        Cursor::Crosshair => Shown::Crosshair,
        Cursor::ColResize => Shown::ColResize,
        Cursor::RowResize => Shown::RowResize,
        Cursor::EwResize => Shown::EwResize,
        Cursor::NsResize => Shown::NsResize,
        Cursor::NeswResize => Shown::NeswResize,
        Cursor::NwseResize => Shown::NwseResize,
        Cursor::Wait => Shown::Wait,
        Cursor::Progress => Shown::Progress,
        Cursor::Help => Shown::Help,
        Cursor::ZoomIn => Shown::ZoomIn,
        Cursor::ZoomOut => Shown::ZoomOut,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vapors_elements::ALL_CURSORS;

    #[test]
    fn every_cursor_is_shown_as_the_platform_cursor_of_its_name() {
        for cursor in ALL_CURSORS {
            // When
            let platform = shown(cursor);

            // Then
            assert_eq!(format!("{platform:?}"), format!("{cursor:?}"));
        }
    }
}
