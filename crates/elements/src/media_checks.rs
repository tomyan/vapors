//! The refusals of the media a document draws: images, surfaces and svg
//! icons (`img_checks`, `surfaces`, `svg_checks`), run after styling.

use crate::node_map::NodeMap;

use vapors_dom::NodeId;

use crate::document::Document;
use crate::error::ElementError;
use crate::style::Used;

/// The refusals of the images, surfaces and svg icons among `live`,
/// `scale` being the viewport's device pixels per CSS px.
pub(crate) fn refusals(
    doc: &Document,
    live: &[NodeId],
    used: &NodeMap<Used>,
    scale: f32,
) -> Vec<ElementError> {
    let mut errors = crate::img_checks::refusals(&doc.images, &doc.arena, live, used);
    errors.extend(crate::surfaces::refusals(&doc.arena, live));
    errors.extend(crate::svg_checks::refusals(&doc.arena, live, used, scale));
    errors
}
