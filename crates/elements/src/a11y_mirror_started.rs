//! The rows a lane start moved, in a take (4i (2) with 4f-3,
//! `docs/plans/splice-layout.md` section 12). An insert or removal in a
//! framed lane moves the start of every later row; such a row has not
//! moved in its frame, and nothing else about it changed, but its
//! record's bounds, or those of its nearest nodes when it is not a node
//! itself, are from a node above it (`a11y_mirror_bounds.rs`). The update
//! keeps each lane's range (`Touched::started`); the take expands it
//! once and gives each row nothing else touched new bounds alone: no
//! role, name, states or tree read again. A node it cannot place so (one
//! hoisted, one no longer drawn) is read again in full.

use vapors_a11y::AxRect;
use vapors_dom::NodeId;

use crate::a11y_error::AxError;
use crate::a11y_mirror_diff::Diff;
use crate::a11y_mirror_index::AxIndex;
use crate::document::Document;
use crate::layout::Key;
use crate::node_map::{NodeMap, NodeSet};

/// What a row's nodes are placed with: the index, the nodes inserted
/// since the last take, the content origins of the parents seen, and the
/// walk's buffers, reused row to row.
pub(crate) struct Rebounding<'a> {
    pub(crate) index: &'a AxIndex,
    pub(crate) replaced: &'a NodeSet,
    origins: NodeMap<[f32; 2]>,
    below: Vec<NodeId>,
    found: Vec<NodeId>,
}

impl<'a> Rebounding<'a> {
    pub(crate) fn new(index: &'a AxIndex, replaced: &'a NodeSet) -> Self {
        Self {
            index,
            replaced,
            origins: NodeMap::default(),
            below: Vec::new(),
            found: Vec::new(),
        }
    }
}

impl Document {
    /// The nodes of the rows each of `started` names, each once: the
    /// lane's children from the first on as the boxes stand now, or the
    /// container's children when the lane or that child is gone.
    pub(crate) fn ax_started_rows(&self, started: &[(Key, Key)]) -> Vec<NodeId> {
        let mut rows = Vec::new();
        let mut seen = NodeSet::default();
        for &range in started {
            let nodes = self.boxes.lane_nodes_from(range).unwrap_or_else(|| {
                let container = range.0.node();
                if self.contains(container) {
                    self.children(container)
                } else {
                    Vec::new()
                }
            });
            rows.extend(nodes.into_iter().filter(|&node| seen.insert(node)));
        }
        rows
    }

    /// Each of `rows` placed again, its record's bounds, or its nearest
    /// nodes' when it is not a node, set from where it is now: a row the
    /// take read in full is skipped only when it is a node (its nodes'
    /// bounds are from it); one that is not is read without its nodes,
    /// so they are placed here whatever else touched the row. Each node
    /// once, none the take read; and the hoisted nodes under the rows,
    /// whose bounds are from the viewport, read in full. The nodes given
    /// bounds alone, and those read in full. `ranges`: how many lane
    /// ranges the rows are from; a node is found under two rows only
    /// when ranges of two lanes overlap (a lane inside a row of another),
    /// or by the hoisted pass, so the nodes done are noted only then.
    ///
    /// # Errors
    /// As [`Document::take_ax_update`], from a node read again in full.
    pub(crate) fn ax_rebound_rows(
        &self,
        (rows, ranges): (&[NodeId], usize),
        read: &NodeSet,
        (rebounding, diff): (&mut Rebounding<'_>, &mut Diff),
    ) -> Result<(usize, usize), AxError> {
        let dedupe = ranges > 1 || !self.ax.hoisted.is_empty();
        let mut done = NodeSet::default();
        let mut counts = (0, 0);
        for &row in rows {
            if read.contains(&row) && self.ax.records.contains_key(&row) {
                continue;
            }
            let mut found = std::mem::take(&mut rebounding.found);
            self.ax_nearest_records(row, (&mut rebounding.below, &mut found));
            for &node in &found {
                if !read.contains(&node) && (!dedupe || done.insert(node)) {
                    let full = self.ax_rebound(node, rebounding, diff)?;
                    *if full { &mut counts.1 } else { &mut counts.0 } += 1;
                }
            }
            rebounding.found = found;
        }
        counts.1 += self.ax_hoisted_rows(rows, (read, &mut done), (rebounding, diff))?;
        Ok(counts)
    }

    /// The hoisted nodes under any of `rows` that neither the take nor the
    /// rows' pass read, read in full; how many.
    fn ax_hoisted_rows(
        &self,
        rows: &[NodeId],
        (read, done): (&NodeSet, &mut NodeSet),
        (rebounding, diff): (&mut Rebounding<'_>, &mut Diff),
    ) -> Result<usize, AxError> {
        if rows.is_empty() || self.ax.hoisted.is_empty() {
            return Ok(0);
        }
        let rows: NodeSet = rows.iter().copied().collect();
        let mut count = 0;
        for node in self.ax_hoisted_under(&rows) {
            if !read.contains(&node) && done.insert(node) {
                let (index, replaced) = (rebounding.index, rebounding.replaced);
                self.ax_classify(node, index, replaced, diff)?;
                count += 1;
            }
        }
        Ok(count)
    }

    /// Into `found` (cleared first): `row` when the mirror holds a record
    /// of it, else the nearest nodes under it it holds records of.
    fn ax_nearest_records(
        &self,
        row: NodeId,
        (below, found): (&mut Vec<NodeId>, &mut Vec<NodeId>),
    ) {
        found.clear();
        if !self.contains(row) {
            return;
        }
        below.push(row);
        while let Some(node) = below.pop() {
            if self.ax.records.contains_key(&node) {
                found.push(node);
            } else {
                below.extend(self.arena.each_child(node));
            }
        }
    }

    /// `node`'s record with its bounds from where its box is now, noted
    /// in `diff` when they differ; read again in full when it is hoisted
    /// or not drawn now. Whether it was read in full.
    fn ax_rebound(
        &self,
        node: NodeId,
        rebounding: &mut Rebounding<'_>,
        diff: &mut Diff,
    ) -> Result<bool, AxError> {
        let old = &self.ax.records[&node];
        let painted = self.painted.as_ref().expect("read from the last update");
        let drawn = painted
            .border_box(node)
            .filter(|_| painted.is_visible(node));
        let Some(rect) = drawn.filter(|_| !old.hoisted) else {
            let (index, replaced) = (rebounding.index, rebounding.replaced);
            return self.ax_classify(node, index, replaced, diff).map(|()| true);
        };
        let origin = match old.parent {
            Some(parent) => *rebounding
                .origins
                .entry(parent)
                .or_insert_with(|| self.ax_content_origin(parent)),
            None => [0.0, 0.0],
        };
        let bounds = AxRect {
            x: rect.x - origin[0],
            y: rect.y - origin[1],
            width: rect.width,
            height: rect.height,
        };
        if bounds != old.bounds {
            let mut new = old.clone();
            new.bounds = bounds;
            diff.rebounded(new);
        }
        Ok(false)
    }
}
