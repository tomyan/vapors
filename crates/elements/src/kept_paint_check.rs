//! The debug build's check on a kept paint (`kept_paint.rs`): the
//! update paints again and compares every part of the two paints but
//! `from`, which the update sets after, the kept boxes, and the counts;
//! the kept `rects` and `paddings` maps name their first differing entry.

use std::fmt::Debug;
use std::rc::Rc;

use vapors_dom::NodeId;

use crate::node_map::NodeMap;
use crate::painted::Painted;

/// The names of the fields of `Painted` that differ between `$a` and
/// `$b`.
macro_rules! differing {
    ($a:expr, $b:expr; $($field:ident),*) => {
        [$((stringify!($field), $a.$field == $b.$field)),*]
            .into_iter()
            .filter_map(|(name, same)| (!same).then_some(name))
            .collect::<Vec<&str>>()
    };
}

/// Panics, naming each part that differs, when `kept` is not `fresh`.
pub(crate) fn assert_same(kept: &Painted, fresh: &Painted) {
    let parts = differing_parts(kept, fresh);
    assert!(
        parts.is_empty(),
        "the kept paint differs from a new paint in {parts:?}: a paint input is not \
         compared in PaintedFrom (crates/elements/src/kept_paint.rs) or BoxFrom \
         (crates/elements/src/kept_box.rs), a copy renumbers wrongly \
         (paint_partial.rs), or a changed slot or shape change is missed \
         (changed_slots.rs, kept_maps.rs, kept_order.rs); reproduce with `cargo xtask test`"
    );
}

/// The parts of `kept` that differ from `fresh`'s; each map's with its
/// first differing entry.
// Exact: a paint made from the same inputs has the same scale, bit for bit.
#[allow(clippy::float_cmp)]
fn differing_parts(kept: &Painted, fresh: &Painted) -> Vec<String> {
    let mut parts: Vec<String> = differing!(kept, fresh; canvas, canvas_from, scale, scene,
        visible, hits, cursors, images, icons, surfaces, ranges, text_origins)
    .into_iter()
    .map(String::from)
    .collect();
    if !same_layouts(&kept.texts, &fresh.texts) {
        parts.push("texts".into());
    }
    parts.extend(first_differing("rects", &kept.rects, &fresh.rects));
    parts.extend(first_differing("paddings", &kept.paddings, &fresh.paddings));
    parts.extend(crate::kept_order_check::differing(
        &kept.order,
        &fresh.order,
    ));
    parts
}

/// The first node, in node order, whose entry in the kept map `name`
/// differs from the fresh map's, with both entries.
fn first_differing<V: PartialEq + Debug>(
    name: &str,
    kept: &NodeMap<V>,
    fresh: &NodeMap<V>,
) -> Option<String> {
    let node = first_differing_node(kept, fresh)?;
    let (kept, fresh) = (kept.get(node), fresh.get(node));
    Some(format!(
        "{name} at {node:?}: kept {kept:?}, fresh {fresh:?}"
    ))
}

/// The first node, in node order, whose entry in `kept` differs from
/// `fresh`'s.
fn first_differing_node<'a, V: PartialEq>(
    kept: &'a NodeMap<V>,
    fresh: &'a NodeMap<V>,
) -> Option<&'a NodeId> {
    if kept == fresh {
        return None;
    }
    let mut nodes: Vec<&NodeId> = kept.keys().chain(fresh.keys()).collect();
    nodes.sort_unstable();
    nodes
        .into_iter()
        .find(|node| kept.get(node) != fresh.get(node))
}

/// Whether each text run is the same layout: a layout kept in the text
/// cache is shared, not copied, by every paint that uses it.
fn same_layouts(kept: &[Rc<vapors_text::Layout>], fresh: &[Rc<vapors_text::Layout>]) -> bool {
    kept.len() == fresh.len() && kept.iter().zip(fresh).all(|(a, b)| Rc::ptr_eq(a, b))
}
