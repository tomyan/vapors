//! Whether an element can take focus now, decided from its attributes
//! and from computed styles brought up to date for it and its
//! ancestors, without layout, as Chromium's `focus()` forces a style
//! update: an element with `display: none` on it or an ancestor, or
//! `visibility` other than `visible`, cannot. The styles are computed
//! from the tree as it is, at the last update's viewport, and kept
//! nowhere, so the next update restyles what it would have anyway.

use vapors_css::cascade::ComputedStyle;
use vapors_dom::NodeId;

use crate::document::Document;
use crate::update::environment;

/// What [`Document::focus`] did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Focus {
    /// Focus moved, or the element already had it.
    Done,
    /// The element cannot take focus now: it is not focusable (not a
    /// control, `a` with `href` or element with `tabindex`), is
    /// disabled, has `display: none` on it or an ancestor, or is not
    /// `visibility: visible`. Nothing happened, as in Chromium.
    NotFocusable,
    /// Its styles cannot be computed yet: no update has run, so there
    /// is no viewport, or a style does not compute (an unresolved
    /// `var()`; the update reports it). The request is kept, and the
    /// next [`Document::settle_focus`] applies it by the same rule, or
    /// drops it with no event; a later `focus` replaces or clears it.
    Pending,
}

impl Document {
    /// Whether `node` can take focus now ([`Focus::Done`] when it can).
    pub(crate) fn focusability(&self, node: NodeId) -> Focus {
        if !self.contains(node) || !self.focus_candidate(node) {
            return Focus::NotFocusable;
        }
        let Some(styles) = self.chain_styles(node) else {
            return Focus::Pending;
        };
        let hidden = styles
            .iter()
            .any(|style| keyword_of(style, "display") == "none");
        let visible = styles
            .last()
            .is_some_and(|style| keyword_of(style, "visibility") == "visible");
        if hidden || !visible {
            Focus::NotFocusable
        } else {
            Focus::Done
        }
    }

    /// The computed styles of `node`'s element ancestors and `node`,
    /// root first, from the tree as it is; `None` without a viewport
    /// or when one does not compute.
    fn chain_styles(&self, node: NodeId) -> Option<Vec<ComputedStyle>> {
        let environment = environment(self.kept.as_ref()?.viewport);
        let mut styles: Vec<ComputedStyle> = Vec::new();
        for element in self.chain(node).into_iter().rev() {
            let style = self
                .sheets
                .compute_element(&self.arena, element, styles.last(), &environment)
                .ok()?;
            styles.push(style);
        }
        Some(styles)
    }
}

/// `property`'s keyword in `style`; empty when it is not a keyword.
fn keyword_of<'a>(style: &'a ComputedStyle, property: &str) -> &'a str {
    style
        .get_by_name(property)
        .and_then(|value| crate::style::keyword(value).ok())
        .unwrap_or("")
}
