//! The accessibility projection's first slice (5-A1's start): each
//! rendered element with one of the asked roles, in document order,
//! with its name and the properties a combobox's keys change, written
//! as Chromium's accessibility tree writes them (the events oracle's
//! `ax_projection`): `activedescendant` and `controls` as the related
//! node's role and name (left out when it is not rendered, as Chromium
//! leaves it out), `expanded` from `aria-expanded`, `focused` on the
//! focused element, `selected=true` from `aria-selected="true"`.
//! Roles are `role`'s first token, else implicit for a text field and a
//! button (`label`, `ul` and `li` arrive in F4); names are `aria-label`,
//! `aria-labelledby`, or the content of an option or button.

use vapors_dom::NodeId;

use crate::document::Document;

/// One node of the projection.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AxNode {
    /// Its role.
    pub role: String,
    /// Its accessible name; empty for none.
    pub name: String,
    /// Its properties as `name=value`, sorted.
    pub properties: Vec<String>,
}

/// Roles whose name is their content.
const NAMED_BY_CONTENT: [&str; 2] = ["option", "button"];

impl Document {
    /// The projection to `roles`, from the last update's styles.
    ///
    /// # Errors
    /// The document has not been updated, so what is rendered is unknown.
    pub fn ax_projection(&self, roles: &[&str]) -> Result<Vec<AxNode>, String> {
        if self.kept.is_none() {
            return Err("the a11y projection needs an update first, for what is rendered".into());
        }
        Ok(self
            .elements()
            .into_iter()
            .filter(|node| self.rendered(*node))
            .filter_map(|node| {
                let role = self.ax_role(node)?;
                roles
                    .contains(&role.as_str())
                    .then(|| self.ax_node(node, role))
            })
            .collect())
    }

    fn ax_node(&self, node: NodeId, role: String) -> AxNode {
        let mut properties = Vec::new();
        for (attribute, property) in [
            ("aria-activedescendant", "activedescendant"),
            ("aria-controls", "controls"),
        ] {
            if let Some(related) = self.related(node, attribute) {
                properties.push(format!("{property}={related}"));
            }
        }
        if let Some(expanded) = self.attribute(node, "aria-expanded") {
            properties.push(format!("expanded={expanded}"));
        }
        if self.active_element() == Some(node) {
            properties.push("focused=true".to_owned());
        }
        if self.attribute(node, "aria-selected") == Some("true") {
            properties.push("selected=true".to_owned());
        }
        properties.sort();
        AxNode {
            name: self.ax_name(node, &role),
            role,
            properties,
        }
    }

    /// `node`'s role: its `role` attribute's first token, else implicit.
    fn ax_role(&self, node: NodeId) -> Option<String> {
        if let Some(role) = self
            .attribute(node, "role")
            .and_then(|r| r.split_whitespace().next())
        {
            return Some(role.to_owned());
        }
        let implicit = match self.tag(node)? {
            "input" if self.is_text_input(node) => "textbox",
            "button" => "button",
            _ => return None,
        };
        Some(implicit.to_owned())
    }

    /// The related node `attribute` names, as role and name, when it is
    /// rendered.
    fn related(&self, node: NodeId, attribute: &str) -> Option<String> {
        let target = self.by_id(self.attribute(node, attribute)?)?;
        if !self.rendered(target) {
            return None;
        }
        let role = self.ax_role(target)?;
        let name = self.ax_name(target, &role);
        Some(format!("{role} \"{name}\""))
    }

    fn ax_name(&self, node: NodeId, role: &str) -> String {
        if let Some(label) = self.attribute(node, "aria-label") {
            return label.trim().to_owned();
        }
        if let Some(ids) = self.attribute(node, "aria-labelledby") {
            let parts: Vec<String> = ids
                .split_whitespace()
                .filter_map(|id| self.by_id(id))
                .map(|n| self.content(n))
                .collect();
            return parts.join(" ");
        }
        if NAMED_BY_CONTENT.contains(&role) {
            return self.content(node);
        }
        String::new()
    }

    /// The element with `id`.
    fn by_id(&self, id: &str) -> Option<NodeId> {
        self.elements()
            .into_iter()
            .find(|n| self.attribute(*n, "id") == Some(id))
    }

    /// `node`'s text content, whitespace collapsed.
    fn content(&self, node: NodeId) -> String {
        let mut text = String::new();
        self.collect_text(node, &mut text);
        text.split_whitespace().collect::<Vec<_>>().join(" ")
    }

    fn collect_text(&self, node: NodeId, out: &mut String) {
        if let Some(own) = self.text(node) {
            out.push_str(own);
            out.push(' ');
        }
        for child in self.children(node) {
            self.collect_text(child, out);
        }
    }

    /// Whether `node` and its ancestors are laid out (no `display: none`).
    fn rendered(&self, node: NodeId) -> bool {
        let Some(kept) = self.kept.as_ref() else {
            return false;
        };
        let mut at = Some(node);
        while let Some(current) = at {
            if kept
                .used
                .get(&current)
                .is_some_and(|used| used.layout.display == taffy::Display::None)
            {
                return false;
            }
            at = self.parent(current);
        }
        true
    }
}
