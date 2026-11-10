//! The tooltip a node shows, as Chromium finds and shows it: the title
//! of the node or its nearest ancestor with a `title` attribute
//! (Blink's `HitTestResult::Title`; `title=""` is present, so it stops
//! the walk and shows nothing), with whitespace at either end trimmed
//! (views' `TooltipStateManager::Show`, `base::TrimWhitespace`, the
//! same set as Rust's `char::is_whitespace`) and each tab as eight
//! spaces (views' `TooltipViewAura::SetText`). Line breaks stay.

use vapors_dom::NodeId;

use crate::document::Document;

impl Document {
    /// The element whose title `node` shows as a tooltip, and the text;
    /// `None` when no element from `node` up has a `title`, the nearest
    /// one's is empty or only whitespace, or `node` is not in the
    /// document.
    #[must_use]
    pub fn tooltip_text(&self, node: NodeId) -> Option<(NodeId, String)> {
        let titled = self.nearest_titled(node)?;
        let text = shown(self.attribute(titled, "title")?);
        (!text.is_empty()).then_some((titled, text))
    }

    fn nearest_titled(&self, node: NodeId) -> Option<NodeId> {
        let mut at = Some(node).filter(|node| self.contains(*node));
        while let Some(node) = at {
            if self.attribute(node, "title").is_some() {
                return Some(node);
            }
            at = self.parent(node);
        }
        None
    }
}

/// `title` as the tooltip shows it.
fn shown(title: &str) -> String {
    title.trim().replace('\t', "        ")
}
