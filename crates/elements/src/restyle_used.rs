//! The used style of one element in a restyle: derived from its kept
//! computed style and its parent's, and kept as an `Rc` (`UsedMap`).

use std::rc::Rc;

use vapors_dom::NodeId;

use crate::document::Document;
use crate::error::ElementError;
use crate::style::{Context, ElementStyles, used};
use crate::update::Viewport;

impl Document {
    /// Keeps `node`'s used style, or its refusals, from its kept computed
    /// style and its parent's.
    pub(crate) fn keep_used(&mut self, node: NodeId, viewport: Viewport) {
        if !crate::check::implemented(&self.arena, node) {
            return;
        }
        let kept = self.kept.as_ref().expect("restyle sets it");
        let parent_node = self.parent(node);
        let parent_used = parent_node.and_then(|p| kept.used.get(&p));
        let context = Context {
            initial: &kept.initial,
            flex_item: parent_used.is_some_and(|p| blockifies(p.layout.display)),
            in_inline: parent_used.is_some_and(|p| p.inline),
            text_like_field: self.arena.get(node, "restyle").element().is_some_and(|e| {
                e.tag == "input"
                    && (crate::check::is_type(e, "number") || crate::text_field::is_text_input(e))
            }),
            surface: crate::surfaces::is_surface(&self.arena, node),
            to_viewport: crate::style::to_viewport(
                node == self.root(),
                node == self.body(),
                kept.used.get(&self.root()).is_none_or(|root| !root.clips()),
            ),
            scale: viewport.scale,
        };
        let styles = ElementStyles {
            own: kept.styles.of(node),
            parent: parent_node.map(|p| kept.styles.of(p)),
        };
        let result = used(styles, &context);
        let refused: Vec<ElementError> = match &result {
            Ok(_) => Vec::new(),
            Err(refusals) => self.unhonoured(node, refusals.clone()).collect(),
        };
        let kept = self.kept.as_mut().expect("restyle sets it");
        kept.refused.insert(node, refused);
        match result {
            Ok(style) => drop(kept.used.insert(node, Rc::new(style))),
            Err(_) => drop(kept.used.remove(&node)),
        }
    }
}

/// Whether a parent of `display` blockifies its children: a flex or
/// grid container does; under `display: none` the children have no box
/// and whether the parent will be a container once shown is unknown, so
/// they are checked as blockified, and checked again when it is shown
/// (its computed style changes, which restyles them).
fn blockifies(display: taffy::Display) -> bool {
    crate::style::is_container(display) || display == taffy::Display::None
}
