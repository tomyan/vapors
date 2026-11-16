//! The index a take reads names through (R6): each id to its first
//! element, and each id to the elements naming it in a [`REFERRING`]
//! attribute. Built by a scan of the document, then patched for the
//! nodes a take touched while every patch is certain: a node whose id
//! is held by another element too (the first in document order wins,
//! which only a scan can tell) asks for a scan. The ids whose element
//! changed are reported, so the nodes naming them are read again.

use std::collections::HashMap;

use vapors_dom::NodeId;

use crate::a11y_roles::Ids;
use crate::document::Document;
use crate::node_map::{NodeMap, NodeSet};

/// The attributes naming other elements by id, whose owner's name or
/// description changes with the named element's content.
pub(crate) const REFERRING: [&str; 2] = ["aria-labelledby", "aria-describedby"];

/// The attributes an element's entry is read from: writing one notes the
/// element for the next take's patch (`Touched::indexed`).
pub(crate) const INDEXED: [&str; 3] = ["id", REFERRING[0], REFERRING[1]];

pub(crate) struct AxIndex {
    pub(crate) ids: Ids,
    pub(crate) referrers: HashMap<String, Vec<NodeId>>,
    /// How many elements carry each id.
    holders: HashMap<String, usize>,
    /// What each element put in: its id, and the ids it names.
    by_node: NodeMap<Entry>,
}

#[derive(Default, PartialEq, Eq)]
struct Entry {
    id: Option<String>,
    names: Vec<String>,
}

/// Whether a scan reads `node`: an element in the tree under the root,
/// not one freed or not yet inserted.
fn scanned(doc: &Document, node: NodeId) -> bool {
    doc.attached(node) && doc.tag(node).is_some()
}

impl Entry {
    fn of(doc: &Document, node: NodeId) -> Self {
        if !scanned(doc, node) {
            return Self::default();
        }
        let names = REFERRING
            .iter()
            .filter_map(|attribute| doc.attribute(node, attribute))
            .flat_map(str::split_whitespace)
            .map(str::to_owned)
            .collect();
        Self {
            id: doc.attribute(node, "id").map(str::to_owned),
            names,
        }
    }
}

impl AxIndex {
    /// The index of `doc`'s elements now: a scan.
    pub(crate) fn of(doc: &Document) -> Self {
        let mut index = Self {
            ids: Ids::new(),
            referrers: HashMap::new(),
            holders: HashMap::new(),
            by_node: NodeMap::default(),
        };
        for node in doc.elements() {
            index.add(node, Entry::of(doc, node));
        }
        index
    }

    /// Patches the entries of `touched`, each read again; the ids whose
    /// element changed. A node a scan does not read (one no longer in the
    /// document, or a text node) has any entry dropped and none added. `None` when a patch cannot be
    /// certain (an id carried by more than one element, counting those
    /// patched together), which asks for a scan.
    pub(crate) fn patch(&mut self, doc: &Document, touched: &NodeSet) -> Option<Vec<String>> {
        let differing: Vec<(NodeId, Entry)> = touched
            .iter()
            .map(|&node| (node, Entry::of(doc, node)))
            .filter(|(node, now)| self.differs(doc, *node, now))
            .collect();
        let certain = differing.iter().all(|(node, now)| self.certain(*node, now));
        if !certain || ids_repeat(&differing) {
            return None;
        }
        let mut changed = Vec::new();
        for (node, now) in differing {
            self.drop(node, &mut changed);
            if scanned(doc, node) {
                self.add_changed(node, now, &mut changed);
            }
        }
        Some(changed)
    }

    /// Whether `node`'s entry is to be patched: `now` differs from what it
    /// put in, or a scan would not read it and it holds an entry.
    fn differs(&self, doc: &Document, node: NodeId, now: &Entry) -> bool {
        let held = self.by_node.get(&node);
        if scanned(doc, node) {
            held != Some(now)
        } else {
            held.is_some()
        }
    }

    /// Whether patching `node` to `now` needs no scan: its old id had no
    /// other holder, and its new id has none.
    fn certain(&self, node: NodeId, now: &Entry) -> bool {
        let old_shared = self
            .by_node
            .get(&node)
            .and_then(|was| was.id.as_ref())
            .is_some_and(|id| self.holders.get(id).copied().unwrap_or_default() > 1);
        let new_shared = now
            .id
            .as_ref()
            .is_some_and(|id| self.ids.get(id).is_some_and(|&holder| holder != node));
        !old_shared && !new_shared
    }

    /// Panics, naming what differs, when this patched index is not
    /// `scanned`, a scan of the same document: a patch missed an element
    /// whose entry changed, such as one in an inserted or removed
    /// subtree (`ax_index_nodes`). Referrers compare as sets, as a patch
    /// appends where a scan goes in document order.
    #[cfg(debug_assertions)]
    pub(crate) fn assert_scanned(&self, scanned: &Self) {
        let differs = if self.ids != scanned.ids {
            Some("the element of an id")
        } else if self.holders != scanned.holders {
            Some("the holders of an id")
        } else if self.by_node != scanned.by_node {
            Some("an element's entry")
        } else if referrer_sets(&self.referrers) != referrer_sets(&scanned.referrers) {
            Some("the referrers of an id")
        } else {
            None
        };
        if let Some(what) = differs {
            panic!(
                "the patched a11y index differs from a scan in {what}: a take patched it \
                 without an element whose entry changed (a11y_mirror_diff.rs \
                 `ax_index_nodes`); reproduce with `cargo xtask test`"
            );
        }
    }

    /// Whether any of `moved`, elements put elsewhere in the document,
    /// carries an id another element carries too: which of them the id
    /// names follows document order, which only a scan tells.
    pub(crate) fn moves_a_shared_id(&self, moved: &[NodeId]) -> bool {
        let ids = moved
            .iter()
            .filter_map(|node| self.by_node.get(node)?.id.as_ref());
        ids.into_iter()
            .any(|id| self.holders.get(id).copied().unwrap_or_default() > 1)
    }

    /// How many elements put something in.
    pub(crate) fn entries(&self) -> usize {
        self.by_node.len()
    }

    /// The ids whose element differs between `self` and `other`.
    pub(crate) fn changed_ids(&self, other: &Self) -> Vec<String> {
        let keys = self.ids.keys().chain(other.ids.keys());
        let mut changed: Vec<String> = keys
            .filter(|id| self.ids.get(*id) != other.ids.get(*id))
            .cloned()
            .collect();
        changed.sort_unstable();
        changed.dedup();
        changed
    }

    /// The elements naming any of `ids`.
    pub(crate) fn referrers_of(&self, ids: &[String]) -> Vec<NodeId> {
        ids.iter()
            .flat_map(|id| self.referrers.get(id).into_iter().flatten().copied())
            .collect()
    }

    fn add(&mut self, node: NodeId, entry: Entry) {
        if let Some(id) = &entry.id {
            self.ids.entry(id.clone()).or_insert(node);
            *self.holders.entry(id.clone()).or_default() += 1;
        }
        for id in &entry.names {
            self.referrers.entry(id.clone()).or_default().push(node);
        }
        self.by_node.insert(node, entry);
    }

    /// Adds `entry`, whose id (certain) makes `node` its element.
    fn add_changed(&mut self, node: NodeId, entry: Entry, changed: &mut Vec<String>) {
        changed.extend(entry.id.clone());
        self.add(node, entry);
    }

    /// Drops what `node` put in; its id (certain) had no other holder.
    fn drop(&mut self, node: NodeId, changed: &mut Vec<String>) {
        let Some(was) = self.by_node.remove(&node) else {
            return;
        };
        if let Some(id) = was.id {
            self.holders.remove(&id);
            self.ids.remove(&id);
            changed.push(id);
        }
        for id in was.names {
            if let Some(referrers) = self.referrers.get_mut(&id) {
                referrers.retain(|&referrer| referrer != node);
            }
        }
    }
}

/// Each id's referrers as a sorted list, ids with none left out.
#[cfg(debug_assertions)]
fn referrer_sets(referrers: &HashMap<String, Vec<NodeId>>) -> Vec<(&String, Vec<NodeId>)> {
    let mut sets: Vec<(&String, Vec<NodeId>)> = referrers
        .iter()
        .filter(|(_, nodes)| !nodes.is_empty())
        .map(|(id, nodes)| {
            let mut nodes = nodes.clone();
            nodes.sort_unstable_by_key(|node| node.0);
            (id, nodes)
        })
        .collect();
    sets.sort_unstable_by(|a, b| a.0.cmp(b.0));
    sets
}

/// Whether two of `entries`, patched together, carry one id: which of
/// them it names is a matter of document order, which only a scan tells.
fn ids_repeat(entries: &[(NodeId, Entry)]) -> bool {
    let mut seen = std::collections::HashSet::new();
    let ids = entries.iter().filter_map(|(_, entry)| entry.id.as_ref());
    ids.into_iter().any(|id| !seen.insert(id))
}
