//! The style of one element in a restyle (analysis PR 2): a keyed
//! element whose key is shareable takes the share table's style for
//! its share key, or computes one and puts it there; any other element
//! computes its own, as before. Debug builds compute every shared
//! element's style per element as well and panic on a difference,
//! naming the template site.

use std::collections::BTreeMap;
use std::rc::Rc;

use vapors_css::cascade::unshareable;
use vapors_css::selector::Element as _;
use vapors_dom::{NodeId, SCOPE_ATTRIBUTE, StyleKey};

use crate::css_element::ElementRef;
use crate::document::Document;
use crate::error::ElementError;
use crate::style::{Counted, ShareKey};
use crate::update::{Viewport, environment};
use crate::update_counts::UpdateCounts;

/// Where an element's style came from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Path {
    /// Taken from the share table.
    Shared,
    /// Computed and put in the share table.
    Computed,
    /// Computed for this element: created without a key.
    Unkeyed,
    /// Computed for this element: one of its key's candidate rules
    /// reads siblings or descendants, or sits under `@container`.
    Unshareable,
}

impl Document {
    /// `node`'s computed style for this update, and how it was found.
    pub(crate) fn style_for(
        &mut self,
        node: NodeId,
        viewport: Viewport,
    ) -> Result<(Rc<Counted>, Path), ElementError> {
        let Some(key) = self
            .arena
            .get(node, "restyle")
            .element()
            .and_then(|e| e.key)
        else {
            return Ok((self.compute_own(node, viewport)?, Path::Unkeyed));
        };
        if !self.shareable(key) {
            return Ok((self.compute_own(node, viewport)?, Path::Unshareable));
        }
        self.shared_style(node, key, viewport)
    }

    /// The share table's style for `node`'s share key, or one computed
    /// for `node` and put there.
    fn shared_style(
        &mut self,
        node: NodeId,
        key: &'static StyleKey,
        viewport: Viewport,
    ) -> Result<(Rc<Counted>, Path), ElementError> {
        let share_key = self.share_key(node, key);
        let kept = self.kept.as_ref().expect("restyle sets it");
        if let Some(style) = kept.shared.take(&share_key) {
            if cfg!(debug_assertions) {
                self.check_shared(node, key, &style, viewport)?;
            }
            return Ok((style, Path::Shared));
        }
        let style = self.compute_own(node, viewport)?;
        let parent = self.parent_style(node);
        let kept = self.kept.as_mut().expect("restyle sets it");
        kept.shared.insert(share_key, Rc::clone(&style), parent);
        Ok((style, Path::Computed))
    }

    fn compute_own(&self, node: NodeId, viewport: Viewport) -> Result<Rc<Counted>, ElementError> {
        let kept = self.kept.as_ref().expect("restyle sets it");
        let parent = self.parent(node).map(|p| kept.styles.of(p));
        let style =
            self.sheets
                .compute_element(&self.arena, node, parent, &environment(viewport))?;
        Ok(kept.styles.alloc(style))
    }

    fn parent_style(&self, node: NodeId) -> Option<Rc<Counted>> {
        let kept = self.kept.as_ref().expect("restyle sets it");
        self.parent(node)
            .and_then(|p| kept.styles.rc(p))
            .map(Rc::clone)
    }

    /// Whether `key`'s elements may share: decided once per `Kept` from
    /// the rules any of them can match.
    fn shareable(&mut self, key: &'static StyleKey) -> bool {
        let address = std::ptr::from_ref(key) as usize;
        let kept = self.kept.as_mut().expect("restyle sets it");
        if let Some(decided) = kept.shared.shareable.get(&address) {
            return *decided;
        }
        let classes: Vec<&str> = key.classes.iter().chain(key.toggles).copied().collect();
        let decided = !self
            .sheets
            .rules_for_key(key.tag, &classes)
            .any(|rule| unshareable(rule).is_some());
        kept.shared.shareable.insert(address, decided);
        decided
    }

    /// The share key of `node`: its template key, its parent's kept
    /// style, its states, its toggles and its selected attributes.
    fn share_key(&self, node: NodeId, key: &'static StyleKey) -> ShareKey {
        let data = self
            .arena
            .get(node, "restyle")
            .element()
            .expect("an element");
        ShareKey {
            key: std::ptr::from_ref(key) as usize,
            parent: self
                .parent_style(node)
                .map_or(0, |p| Rc::as_ptr(&p) as usize),
            states: ElementRef::new(&self.arena, node).states(),
            toggles: data.toggles,
            attributes: self.selected_attributes(&data.attributes),
        }
    }

    /// The attributes of `attributes` a sheet selects, but `class` and
    /// the scope attribute, which the template key fixes.
    fn selected_attributes(
        &self,
        attributes: &BTreeMap<String, String>,
    ) -> Box<[(String, String)]> {
        let selected = self.sheets.selected_attributes();
        attributes
            .iter()
            .filter(|(name, _)| {
                selected.contains(*name)
                    && name.as_str() != "class"
                    && name.as_str() != SCOPE_ATTRIBUTE
            })
            .map(|(name, value)| (name.clone(), value.clone()))
            .collect()
    }

    /// The oracle: `shared` must equal the style computed for `node`
    /// alone.
    fn check_shared(
        &self,
        node: NodeId,
        key: &StyleKey,
        shared: &Counted,
        viewport: Viewport,
    ) -> Result<(), ElementError> {
        let own = self.compute_own(node, viewport)?;
        let differing = crate::restyle_check::differing_property(Some(shared), &own);
        assert!(
            **own == **shared,
            "the shared style of {} differs from its own: {differing}; the share key (restyle_share.rs) misses an input of a selector that reaches <{}> at {}. Reproduce with the failing test",
            self.describe(node),
            key.tag,
            key.site
        );
        Ok(())
    }
}

impl UpdateCounts {
    /// Notes where one restyled element's style came from.
    pub(crate) fn count_style(&mut self, path: Path) {
        let count = match path {
            Path::Shared => &mut self.styles_shared,
            Path::Computed => &mut self.styles_computed,
            Path::Unkeyed => &mut self.styles_unkeyed,
            Path::Unshareable => &mut self.styles_unshareable,
        };
        *count += 1;
    }

    /// Drops a failed restyle's partial style counts, as
    /// `elements_restyled` drops its own.
    pub(crate) fn forget_styles(&mut self) {
        self.styles_shared = 0;
        self.styles_computed = 0;
        self.styles_unkeyed = 0;
        self.styles_unshareable = 0;
        self.used_shared = 0;
        self.used_derived = 0;
    }
}

#[cfg(test)]
#[path = "restyle_share_tests.rs"]
mod tests;
