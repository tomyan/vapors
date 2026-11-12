//! What the a11y mirror keeps between takes (R6): the records last
//! emitted, the focused node as last emitted, and the index of ids and
//! of the elements naming others by id. The next slice reads them back
//! to emit only what changed.

use std::collections::HashMap;

use vapors_dom::NodeId;

use crate::a11y_mirror::AxMirrorNode;
use crate::a11y_roles::Ids;
use crate::document::Document;
use crate::node_map::NodeMap;

/// The attributes naming other elements by id, whose owner's name or
/// description changes with the named element's content.
pub(crate) const REFERRING: [&str; 2] = ["aria-labelledby", "aria-describedby"];

#[derive(Default)]
pub(crate) struct AxMirror {
    /// The records last emitted, by node.
    pub(crate) records: NodeMap<AxMirrorNode>,
    /// The focused node as last emitted.
    pub(crate) focus: Option<NodeId>,
    pub(crate) index: Option<AxIndex>,
}

/// Each id to its first element, and each id to the elements naming it
/// in a [`REFERRING`] attribute.
pub(crate) struct AxIndex {
    pub(crate) ids: Ids,
    pub(crate) referrers: HashMap<String, Vec<NodeId>>,
}

impl AxIndex {
    /// The index of `doc`'s elements now.
    pub(crate) fn of(doc: &Document) -> Self {
        let mut index = Self {
            ids: Ids::new(),
            referrers: HashMap::new(),
        };
        for node in doc.elements() {
            if let Some(id) = doc.attribute(node, "id") {
                index.ids.entry(id.to_owned()).or_insert(node);
            }
            for attribute in REFERRING {
                let named = doc.attribute(node, attribute).unwrap_or_default();
                for id in named.split_whitespace() {
                    index.referrers.entry(id.to_owned()).or_default().push(node);
                }
            }
        }
        index
    }
}

impl AxMirror {
    /// Keeps `nodes` as the records of every node, after a take of all.
    pub(crate) fn reset(&mut self, nodes: &[AxMirrorNode], index: AxIndex, focus: Option<NodeId>) {
        self.records = nodes.iter().map(|n| (n.node, n.clone())).collect();
        self.index = Some(index);
        self.focus = focus;
    }
}
