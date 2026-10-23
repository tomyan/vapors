//! Tree walks the incremental tests need, through `Document`'s public
//! accessors.

use vapors_dom::NodeId;
use vapors_elements::Document;

/// Every text node, in tree order.
pub fn text_nodes(doc: &Document) -> Vec<NodeId> {
    let mut out = Vec::new();
    let mut stack = vec![doc.root()];
    while let Some(node) = stack.pop() {
        if doc.tag(node).is_none() {
            out.push(node);
        }
        stack.extend(doc.children(node).into_iter().rev());
    }
    out
}

/// Whether `node` is `ancestor` or below it.
pub fn is_inside(doc: &Document, node: NodeId, ancestor: NodeId) -> bool {
    let mut at = Some(node);
    while let Some(current) = at {
        if current == ancestor {
            return true;
        }
        at = doc.parent(current);
    }
    false
}
