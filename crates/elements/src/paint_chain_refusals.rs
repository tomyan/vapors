//! The boxes whose clip chain is past `MAX_CLIP_CHAIN` (`clip_chain.rs`,
//! flat-scene F3): the update is refused once, after paint, naming the
//! first such box in tree order and counting the others.

use crate::document::Document;
use crate::error::ElementError;
use crate::layout::Placed;

impl Document {
    /// One refusal for `long` (slot, links), naming the first in tree
    /// order.
    ///
    /// # Errors
    /// Any box in `long`.
    pub(crate) fn refuse_long_chains(
        &self,
        long: &[(usize, usize)],
        placed: &Placed,
    ) -> Result<(), Vec<ElementError>> {
        if long.is_empty() {
            return Ok(());
        }
        let order = crate::paint_cause::tree_order(placed);
        let &(slot, links) = order
            .iter()
            .find_map(|&slot| long.iter().find(|(at, _)| *at == slot))
            .unwrap_or(&long[0]);
        let element = placed.slots.key(slot).map_or_else(
            || "a box with no element".to_owned(),
            |key| crate::check::describe(&self.arena, key.node()),
        );
        let others = long.len() - 1;
        Err(vec![ElementError::Refused(message(
            &element,
            (slot, links),
            others,
        ))])
    }
}

fn message(element: &str, (slot, links): (usize, usize), others: usize) -> String {
    let also = match others {
        0 => String::new(),
        n => format!("; {n} other boxes are past it too"),
    };
    format!(
        "{element} (box slot {slot}): its clip chain has {links} links (its own clip, and one \
         per enclosing scroll container or lane row under a clip), past MAX_CLIP_CHAIN = {} \
         (crates/elements/src/clip_chain.rs, measured on the corpus and the demos){also}; fix: \
         nest fewer clipping scroll containers around it; reproduce: update the document, as \
         `cargo test -p vapors-elements --test tests clip_chain` does",
        crate::MAX_CLIP_CHAIN
    )
}
