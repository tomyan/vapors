//! The boxes paint could not place (issue #449): a border box with an
//! edge that is not finite in device pixels, as a width past f32's range
//! gives. Each is refused after paint, naming its element and slot, so
//! no infinite quad is shown; a box is placed, refused, or has no rect
//! and is not painted.

use crate::document::Document;
use crate::error::ElementError;
use crate::layout::{BoxRect, Placed};

impl Document {
    /// One refusal per box in `unresolvable` (slot, CSS rect), at `scale`.
    ///
    /// # Errors
    /// Any box in `unresolvable`.
    pub(crate) fn refuse_unresolvable(
        &self,
        unresolvable: &[(usize, BoxRect)],
        placed: &Placed,
        scale: f32,
    ) -> Result<(), Vec<ElementError>> {
        if unresolvable.is_empty() {
            return Ok(());
        }
        let refuse = |&(slot, rect): &(usize, BoxRect)| {
            let element = placed.slots.key(slot).map_or_else(
                || "a box with no element".to_owned(),
                |key| crate::check::describe(&self.arena, key.node()),
            );
            ElementError::Refused(message(&element, slot, rect, scale))
        };
        Err(unresolvable.iter().map(refuse).collect())
    }
}

fn message(element: &str, slot: usize, rect: BoxRect, scale: f32) -> String {
    format!(
        "{element} (box slot {slot}): its border box (x {}, y {}, width {}, height {} CSS px) \
         has an edge that is not finite in device pixels at scale {scale}, so paint cannot place \
         it and refuses it rather than draw an infinite quad; fix: keep its size and position \
         within f32's range at that scale (about 3.4e38 device px; `width: 1e39px` is past it); \
         reproduce with the update that refused",
        rect.x, rect.y, rect.width, rect.height
    )
}
