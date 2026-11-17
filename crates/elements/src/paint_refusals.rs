//! The boxes paint could not place (issue #449): a border box, an item
//! or a clip with a coordinate that is not finite in device pixels, as a
//! length past f32's range gives. The update is refused once, after
//! paint, naming the cause (`paint_cause.rs`) and counting the other
//! boxes it reaches, so nothing infinite is shown.

use crate::document::Document;
use crate::error::ElementError;
use crate::layout::Placed;

/// Which part of a box held the coordinate.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PartKind {
    /// Its border box.
    BorderBox,
    /// Its own step's items, of `kind` (`vapors_scene::NonFinite`).
    Own(&'static str),
    /// Its outline ring.
    Outline,
    /// Its scrollbar thumbs.
    Thumb,
}

/// A box paint could not place: its slot and what held the coordinate.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct NotPlaced {
    pub(crate) slot: usize,
    pub(crate) part: PartKind,
}

/// What paint refused, for the update to report after paint.
pub(crate) struct Refused {
    /// The boxes it could not place.
    pub(crate) unplaced: Vec<NotPlaced>,
    /// The boxes whose clip chain is past `MAX_CLIP_CHAIN`, with its links.
    pub(crate) long_chains: Vec<(usize, usize)>,
}

impl Document {
    /// One refusal for `unplaced`, at `scale`, naming its cause.
    ///
    /// # Errors
    /// Any box in `unplaced`.
    pub(crate) fn refuse_unplaced(
        &self,
        unplaced: &[NotPlaced],
        placed: &Placed,
        scale: f32,
    ) -> Result<(), Vec<ElementError>> {
        let Some(cause) = crate::paint_cause::cause(unplaced, placed) else {
            return Ok(());
        };
        let element = placed.slots.key(cause.slot).map_or_else(
            || "a box with no element".to_owned(),
            |key| crate::check::describe(&self.arena, key.node()),
        );
        let others = unplaced.iter().filter(|n| n.slot != cause.slot).count();
        Err(vec![ElementError::Refused(
            cause.message(&element, scale, others),
        )])
    }
}
