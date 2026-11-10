//! The a11y tree flattened as the events oracle compares it with
//! Chromium's (the corpus's `ax_projection`): each node with one of the
//! asked roles, in document order, with its name, its value, its
//! description and its states as Chromium writes them, and `activedescendant` and
//! `controls` as the related node's role and name (left out when that
//! node is not in the tree, as Chromium leaves it out).

use std::collections::HashMap;

use vapors_dom::NodeId;

use crate::a11y_error::AxError;
use crate::a11y_roles::Ids;
use crate::a11y_tree::AxElement;
use crate::document::Document;

/// One node of the projection.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AxNode {
    /// Its role.
    pub role: String,
    /// Its accessible name; empty for none.
    pub name: String,
    /// Its value, when it shows one that is not empty.
    pub value: Option<String>,
    /// Its accessible description, when not empty.
    pub description: Option<String>,
    /// Its properties as `name=value`, sorted.
    pub properties: Vec<String>,
}

/// The attributes naming a related node, and the property each writes.
const RELATED: [(&str, &str); 2] = [
    ("aria-activedescendant", "activedescendant"),
    ("aria-controls", "controls"),
];

/// Roles Chromium's tree names otherwise than ARIA: `(ARIA, Chromium)`.
const CHROMIUM_NAMES: [(&str, &str); 1] = [("img", "image")];

/// Chromium's name for `role` when it differs from ARIA's.
#[must_use]
pub fn chromium_name_for_aria(role: &str) -> Option<&'static str> {
    CHROMIUM_NAMES
        .iter()
        .find(|(aria, _)| *aria == role)
        .map(|(_, chromium)| *chromium)
}

/// `role` as Chromium's accessibility tree names it.
fn chromium_role(role: &str) -> &str {
    CHROMIUM_NAMES
        .iter()
        .find(|(aria, _)| *aria == role)
        .map_or(role, |(_, chromium)| chromium)
}

impl Document {
    /// The tree's nodes with one of `roles`, flattened in document order,
    /// roles named as Chromium's tree names them (`image` for ARIA's
    /// `img`).
    ///
    /// # Errors
    /// As [`Document::ax_tree`].
    pub fn ax_projection(&self, roles: &[&str]) -> Result<Vec<AxNode>, AxError> {
        if let Some(asked) = roles.iter().find(|r| chromium_name_for_aria(r).is_some()) {
            return Err(AxError::RoleNamedByAria {
                asked: (*asked).to_owned(),
                chromium: chromium_name_for_aria(asked).unwrap_or_default().to_owned(),
            });
        }
        let tree = self.ax_tree()?;
        let mut all = Vec::new();
        flatten(&tree, &mut all);
        let by_node: HashMap<NodeId, &AxElement> = all.iter().map(|e| (e.node, *e)).collect();
        let ids = self.ax_ids();
        Ok(all
            .iter()
            .filter(|element| roles.contains(&chromium_role(&element.role)))
            .map(|element| self.ax_node(element, &by_node, &ids))
            .collect())
    }

    /// `element` as the oracle writes it, its related nodes found in
    /// `by_node`.
    fn ax_node(
        &self,
        element: &AxElement,
        by_node: &HashMap<NodeId, &AxElement>,
        ids: &Ids,
    ) -> AxNode {
        let mut properties = element.states.set();
        for (attribute, property) in RELATED {
            let related = self
                .attribute(element.node, attribute)
                .and_then(|id| ids.get(id));
            if let Some(related) = related.and_then(|node| by_node.get(node)) {
                properties.push(format!(
                    "{property}={} \"{}\"",
                    chromium_role(&related.role),
                    related.name
                ));
            }
        }
        properties.sort();
        AxNode {
            role: chromium_role(&element.role).to_owned(),
            name: element.name.clone(),
            value: element.value.clone(),
            description: element.description.clone(),
            properties,
        }
    }
}

/// Every node under `nodes`, depth first, in document order.
fn flatten<'t>(nodes: &'t [AxElement], out: &mut Vec<&'t AxElement>) {
    for node in nodes {
        out.push(node);
        flatten(&node.children, out);
    }
}
