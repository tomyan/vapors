//! The used style of one element in a restyle (build-time styles U2):
//! an element whose computed style came from the share table takes the
//! used-style table's `Used` for that style, its parent's `Used` and
//! its context (`style/used_share.rs`), or derives one and puts it
//! there; any other element derives its own. Debug builds derive every
//! taken `Used` for its element as well and panic on a difference,
//! naming the template site and the field.

use std::rc::Rc;

use vapors_dom::NodeId;

use crate::document::Document;
use crate::restyle_share::Path;
use crate::style::{Context, ElementStyles, Refusal, Used, UsedKey, used};
use crate::update::Viewport;

/// How an element's used style was found.
enum Found {
    /// Taken from the used-style table.
    Taken(Rc<Used>),
    /// Derived for the element; the key it goes in the table under, when
    /// its computed style is shared.
    Derived(Result<Rc<Used>, Vec<Refusal>>, Option<UsedKey>),
}

impl Document {
    /// Keeps `node`'s used style, or its refusals, from its kept computed
    /// style and its parent's; `path` says where the computed style came
    /// from.
    pub(crate) fn keep_used(&mut self, node: NodeId, viewport: Viewport, path: Path) {
        if !crate::check::implemented(&self.arena, node) {
            return;
        }
        let found = self.find_used(node, viewport, path);
        let (style, refused) = match self.keep_found(node, found) {
            Ok(style) => (Some(style), Vec::new()),
            Err(refusals) => (None, self.unhonoured(node, refusals).collect()),
        };
        let kept = self.kept.as_mut().expect("restyle sets it");
        kept.refused.insert(node, refused);
        match style {
            Some(style) => drop(kept.used.insert(node, style)),
            None => drop(kept.used.remove(&node)),
        }
    }

    /// The table's used style for `node`, or one derived for it.
    fn find_used(&self, node: NodeId, viewport: Viewport, path: Path) -> Found {
        let context = self.context(node, viewport);
        let styles = self.element_styles(node);
        if !matches!(path, Path::Shared | Path::Computed) {
            return Found::Derived(used(styles, &context).map(Rc::new), None);
        }
        let kept = self.kept.as_ref().expect("restyle sets it");
        let style = kept
            .styles
            .rc(node)
            .expect("restyled before its used style");
        let key = UsedKey::new(style, self.parent_used(node), &context);
        let Some(taken) = kept.shared.used.take(&key) else {
            return Found::Derived(used(styles, &context).map(Rc::new), Some(key));
        };
        if cfg!(debug_assertions) {
            self.check_used(node, &taken, &used(styles, &context));
        }
        Found::Taken(taken)
    }

    /// Counts how `found` was found and puts a derived used style in the
    /// table under its key, pinning what the key's addresses point at.
    fn keep_found(&mut self, node: NodeId, found: Found) -> Result<Rc<Used>, Vec<Refusal>> {
        let (derived, key) = match found {
            Found::Taken(taken) => {
                self.counts.used_shared += 1;
                return Ok(taken);
            }
            Found::Derived(derived, key) => (derived, key),
        };
        self.counts.used_derived += 1;
        let derived = derived?;
        if let Some(key) = key {
            let parent = self.parent_used(node).map(Rc::clone);
            let kept = self.kept.as_mut().expect("restyle sets it");
            let style = Rc::clone(kept.styles.rc(node).expect("restyled"));
            kept.shared
                .used
                .insert(key, Rc::clone(&derived), style, parent);
        }
        Ok(derived)
    }

    /// What `node`'s used style depends on besides its computed styles.
    fn context(&self, node: NodeId, viewport: Viewport) -> Context<'_> {
        let kept = self.kept.as_ref().expect("restyle sets it");
        let parent_used = self.parent_used(node);
        Context {
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
        }
    }

    fn element_styles(&self, node: NodeId) -> ElementStyles<'_> {
        let kept = self.kept.as_ref().expect("restyle sets it");
        ElementStyles {
            own: kept.styles.of(node),
            parent: self.parent(node).map(|p| kept.styles.of(p)),
        }
    }

    fn parent_used(&self, node: NodeId) -> Option<&Rc<Used>> {
        let kept = self.kept.as_ref().expect("restyle sets it");
        self.parent(node).and_then(|p| kept.used.get(&p))
    }

    /// The oracle: `taken` must equal the used style derived for `node`
    /// alone.
    fn check_used(&self, node: NodeId, taken: &Used, own: &Result<Used, Vec<Refusal>>) {
        let difference = match own {
            Ok(own) => own.differing_field(taken).map(|f| format!("field `{f}`")),
            Err(_) => Some("its own is refused".to_owned()),
        };
        let Some(difference) = difference else { return };
        let element = self.arena.get(node, "restyle").element();
        let key = element
            .and_then(|e| e.key)
            .expect("a shared style is keyed");
        panic!(
            "the shared used style of {} differs from its own: {difference}; the used-style key (style/used_share.rs) misses an input of the derivation for <{}> at {}. Reproduce with the failing test",
            self.describe(node),
            key.tag,
            key.site
        );
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

#[cfg(test)]
#[path = "restyle_used_tests.rs"]
mod tests;
