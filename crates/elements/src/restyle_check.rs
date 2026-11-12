//! Debug check of a narrowed restyle (splice.md (b2)): after an update
//! that marked an inserted or removed child's siblings by what the
//! sheets reach, every element's style computed afresh from the root
//! equals the kept one. A difference names the element, the property,
//! and the sheets' sibling-reaching selectors that match it.
//!
//! Debug builds only, and O(elements) per such update: it would be
//! sampled or dropped once the narrowing has run in the corpus tests
//! for a while with no mismatch.

use vapors_css::cascade::ComputedStyle;
use vapors_dom::NodeId;

use crate::check::elements;
use crate::document::Document;
use crate::style::Styles;
use crate::update::{Viewport, environment};

/// What differs between the kept style of an element and its fresh
/// one, for the message: a missing kept style, the first longhand whose
/// values differ (found over every longhand, so one the fresh style
/// leaves unset while the kept style set it is named), or else the part
/// of the style outside the longhands.
pub(crate) fn differing_property(kept: Option<&ComputedStyle>, fresh: &ComputedStyle) -> String {
    let Some(kept) = kept else {
        return "no kept style".to_owned();
    };
    fresh
        .iter()
        .find(|(id, value)| kept.get(*id) != *value)
        .map_or_else(
            || {
                "the longhands are equal; the custom properties, font size, colour or the winning declarations differ"
                    .to_owned()
            },
            |(id, value)| {
                format!(
                    "`{}` is {} fresh, {} kept",
                    id.name(),
                    value.to_css(),
                    kept.get(id).to_css()
                )
            },
        )
}

impl Document {
    /// Panics on the first element whose kept style differs from a
    /// fresh one.
    pub(crate) fn check_narrowed_restyle(&self, viewport: Viewport) {
        let kept = self.kept.as_ref().expect("restyle sets it");
        let env = environment(viewport);
        let mut fresh = Styles::default();
        for node in elements(&self.arena, self.root()) {
            let parent = self.parent(node).map(|p| fresh.of(p));
            let style = self
                .sheets
                .compute_element(&self.arena, node, parent, &env)
                .unwrap_or_else(|error| panic!("the fresh restyle of {node:?} failed: {error}"));
            assert!(
                kept.styles.get(node) == Some(&style),
                "{}",
                self.narrowed_restyle_mismatch(node, kept.styles.get(node), &style)
            );
            let style = fresh.alloc(style);
            fresh.set(node, style);
        }
    }

    fn narrowed_restyle_mismatch(
        &self,
        node: NodeId,
        kept: Option<&ComputedStyle>,
        fresh: &ComputedStyle,
    ) -> String {
        let differing = differing_property(kept, fresh);
        let selectors = self.sheets.sibling_selectors(&self.arena, node);
        let (matching, other): (Vec<_>, Vec<_>) = selectors.into_iter().partition(|s| s.1);
        let named = |list: Vec<(String, bool)>| list.into_iter().map(|s| s.0).collect::<Vec<_>>();
        format!(
            "a narrowed restyle missed {}: {differing}; the sheets' sibling-reaching selectors matching it now are {:?}, the others {:?}. Fix: `SiblingDeps::of_selector` (style/sibling_deps.rs) must reach it; reproduce with the test that inserted or removed the child",
            self.describe(node),
            named(matching),
            named(other)
        )
    }

    /// `tag#id.class.class` for a message.
    pub(crate) fn describe(&self, node: NodeId) -> String {
        let Some(element) = self.arena.get(node, "restyle").element() else {
            return format!("{node:?}");
        };
        let id = element
            .attributes
            .get("id")
            .map(|id| format!("#{id}"))
            .unwrap_or_default();
        let classes: String = element
            .attributes
            .get("class")
            .map(|classes| {
                classes
                    .split_ascii_whitespace()
                    .fold(String::new(), |out, c| out + "." + c)
            })
            .unwrap_or_default();
        format!("<{}{id}{classes}> ({node:?})", element.tag)
    }
}

#[cfg(test)]
#[path = "restyle_check_tests.rs"]
mod tests;
