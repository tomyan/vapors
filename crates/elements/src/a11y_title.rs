//! The `title` attribute in the a11y tree, as Chromium's has it (pinned
//! by the corpus's `titles.events`): the name of a node nothing else
//! names, its whitespace runs collapsed to one space and not trimmed;
//! else the description, collapsed and trimmed, unless the content of
//! the `aria-describedby` ids found describes it. A title of only ASCII
//! whitespace is none; other whitespace (no-break space) is kept.

use vapors_dom::NodeId;

use crate::a11y_roles::Ids;
use crate::document::Document;

impl Document {
    /// `node`'s name from its `title`; empty for none.
    pub(crate) fn ax_title_name(&self, node: NodeId) -> String {
        self.attribute(node, "title")
            .map_or_else(String::new, collapsed)
    }

    /// The placeholder an input shows while empty, as its description
    /// (`docs/placeholder.md`), whether or not it is empty now: the
    /// hint stays the field's description once it has a value. Never
    /// its name: that is the label's, `aria-label`'s or the title's.
    fn ax_placeholder(&self, node: NodeId) -> Option<String> {
        let element = self.arena.get(node, "a11y").element()?;
        let text = crate::placeholder::text_of(element)?;
        Some(text.trim_ascii().to_owned()).filter(|text| !text.is_empty())
    }

    /// `node`'s accessible name and description, one lookup of the
    /// name before its title serving both; the description is `None`
    /// for none.
    pub(crate) fn ax_name_and_description(
        &self,
        node: NodeId,
        role: &str,
        ids: &Ids,
    ) -> (String, Option<String>) {
        let before_title = self.ax_name_before_title(node, role, ids);
        let title = self.ax_title_name(node);
        let described = self.ax_referenced(node, "aria-describedby", ids);
        if !described.is_empty() {
            return (name_or(before_title, title), Some(described));
        }
        // ASCII trim as `collapsed`: Chromium keeps a no-break space at
        // either end (titles.events, `nbspdesc`)
        let description = Some(title.trim_ascii().to_owned())
            .filter(|description| !before_title.is_empty() && !description.is_empty())
            .or_else(|| self.ax_placeholder(node));
        (name_or(before_title, title), description)
    }
}

/// `before_title` when it names the node, else `title`.
fn name_or(before_title: String, title: String) -> String {
    if before_title.is_empty() {
        title
    } else {
        before_title
    }
}

/// `text` with each run of whitespace one space, or empty when it is
/// only whitespace.
fn collapsed(text: &str) -> String {
    if text.trim_ascii().is_empty() {
        return String::new();
    }
    let mut out = String::with_capacity(text.len());
    let mut after_space = false;
    for c in text.chars() {
        let space = c.is_ascii_whitespace();
        if !(space && after_space) {
            out.push(if space { ' ' } else { c });
        }
        after_space = space;
    }
    out
}
