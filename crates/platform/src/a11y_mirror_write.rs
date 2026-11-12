//! What a record writes to its element (R6 PR 5a): the attributes and
//! text that differ from the record the mirror last applied to the
//! node, so a record changed in one attribute is one write and a
//! reconcile of unchanged records writes nothing. A changed scroll
//! offset is reported, not written: the mirror writes offsets once
//! every element is in place (`a11y_mirror_scroll.rs`).

use vapors_a11y::AxMirrorNode;

use crate::a11y_mirror_attrs::{attributes, text};
use crate::a11y_mirror_dom::MirrorDom;

/// Writes what `node` changed since `last` to `element`; everything,
/// for a new element. Whether the scroll offset changed (or is new).
///
/// # Errors
/// The record has no attributes in the mapping (`attributes`), or the
/// mapping gave the two records lists of different lengths, which the
/// comparison relies on: a bug in `a11y_mirror_attrs.rs`.
pub(crate) fn write<D: MirrorDom>(
    dom: &mut D,
    element: &D::Element,
    node: &AxMirrorNode,
    last: Option<&AxMirrorNode>,
) -> Result<bool, String> {
    let now = attributes(node)?;
    let before = last.map(attributes).transpose()?;
    if let Some(before) = &before
        && before.len() != now.len()
    {
        return Err(format!(
            "the a11y mapping gave node {} {} attributes before and {} now; fix: \
             a11y_mirror_attrs::attributes lists every attribute, as a value or none, in one order",
            node.node.0,
            before.len(),
            now.len()
        ));
    }
    for (at, (name, value)) in now.iter().enumerate() {
        if before.as_ref().is_some_and(|before| before[at].1 == *value) {
            continue;
        }
        match value {
            Some(value) => dom.set_attribute(element, name, value),
            None if before.is_some() => dom.remove_attribute(element, name),
            None => {}
        }
    }
    write_text(dom, element, node, last);
    Ok(scroll_changed(node, last))
}

fn write_text<D: MirrorDom>(
    dom: &mut D,
    element: &D::Element,
    node: &AxMirrorNode,
    last: Option<&AxMirrorNode>,
) {
    let now = text(node);
    let before = last.map_or("", text);
    if now != before {
        dom.set_text(element, now);
    }
}

/// Whether `node`'s scroll offset differs from `last`'s, or from the
/// `[0, 0]` a new element reads.
fn scroll_changed(node: &AxMirrorNode, last: Option<&AxMirrorNode>) -> bool {
    let Some(scroll) = node.scroll else {
        return false;
    };
    let before = last
        .and_then(|last| last.scroll)
        .map_or([0.0, 0.0], |scroll| scroll.offset);
    // exact: any change of the offset is one to write
    scroll.offset.map(f32::to_bits) != before.map(f32::to_bits)
}
