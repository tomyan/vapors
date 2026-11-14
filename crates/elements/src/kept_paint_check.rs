//! The debug build's check on a kept paint (`kept_paint.rs`): the
//! update paints again and compares every part of the two paints but
//! `from`, which the update sets after, the kept boxes, and the counts;
//! the hits in painting order, and the hits kept for hit testing
//! against a fresh gather, naming the first that differs; the steps
//! that draw against ones found afresh; the scene item by item with ids
//! resolved, the slabs against the kept boxes' items, and each kept
//! part's slot against paint order (`kept_scene_check.rs`); the kept
//! `rects` and `paddings` maps name their first differing entry.

use std::fmt::Debug;

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
         (crates/elements/src/kept_box.rs), a part's slab ids are not freed \
         or taken once (paint_partial.rs, slab.rs), a changed slot or shape change is missed \
         (changed_slots.rs, kept_maps.rs, kept_order.rs), a splice put a step at the wrong \
         place (order_splice.rs), or a slot's hits were not patched \
         (kept_hits.rs, paint_partial.rs drawn_updated); reproduce with `cargo xtask test`"
    );
}

/// The parts of `kept` that differ from `fresh`'s; each map's with its
/// first differing entry.
// Exact: a paint made from the same inputs has the same scale, bit for bit.
#[allow(clippy::float_cmp)]
fn differing_parts(kept: &Painted, fresh: &Painted) -> Vec<String> {
    let mut parts: Vec<String> = differing!(kept, fresh; canvas, canvas_from, scale,
        visible, cursors, ranges, text_origins)
    .into_iter()
    .map(String::from)
    .collect();
    parts.extend(differing_hits(kept, fresh));
    parts.extend(differing_drawn(kept).map(String::from));
    parts.extend(differing_extents(kept).map(String::from));
    parts.extend(crate::kept_scene_check::first_differing_item(kept, fresh));
    parts.extend(crate::kept_scene_check::part_off_steps(kept));
    parts.extend(crate::kept_scene_check::slab_difference(
        &kept.slabs,
        kept_items(kept),
    ));
    parts.extend(first_differing("rects", &kept.rects, &fresh.rects));
    parts.extend(first_differing("paddings", &kept.paddings, &fresh.paddings));
    parts.extend(first_differing(
        "node frames",
        &kept.node_frames,
        &fresh.node_frames,
    ));
    parts.extend(
        (kept.frames != fresh.frames)
            .then(|| format!("frames: kept {:?}, fresh {:?}", kept.frames, fresh.frames)),
    );
    parts.extend(crate::kept_order_check::differing(
        &kept.order,
        &fresh.order,
    ));
    parts
}

/// "hits" when `kept`'s hits are not `fresh`'s, in painting order; the
/// first kept hit for hit testing that differs from a fresh gather of
/// `kept`'s scene. The kept hits are gathered when none are, so every
/// later update patches them and is compared.
fn differing_hits(kept: &Painted, fresh: &Painted) -> impl Iterator<Item = String> {
    let hits = !kept.scene().hits().eq(fresh.scene().hits());
    let gathered = crate::kept_hits::KeptHits::of(&kept.scene());
    let first = first_differing_hit(kept.kept_hits(), &gathered);
    hits.then(|| "hits".to_owned()).into_iter().chain(first)
}

/// The first place where `kept`'s hits and `gathered`'s differ, with
/// both.
fn first_differing_hit(
    kept: &crate::kept_hits::KeptHits,
    gathered: &crate::kept_hits::KeptHits,
) -> Option<String> {
    let (mut kept, mut gathered) = (kept.hits(), gathered.hits());
    (0..).find_map(|at| match (kept.next(), gathered.next()) {
        (None, None) => Some(None),
        (k, g) if k != g => Some(Some(format!(
            "kept hits at {at}: kept {k:?}, gathered afresh {g:?}"
        ))),
        _ => None,
    })?
}

/// Which total of `kept`'s extents index is not the sum over its
/// entries; else "extents index" when the index is not the one made
/// afresh from its drawing steps.
fn differing_extents(kept: &Painted) -> Option<&'static str> {
    if let Some(total) = kept.extents.total_differing() {
        return Some(total);
    }
    let places = kept.order.steps.len();
    let afresh = crate::frame_extents::Extents::of(&kept.drawn, places, &kept.kept_boxes);
    (kept.extents != afresh).then_some("extents index")
}

/// "drawn steps" when the steps `kept` lists as drawing are not those
/// found afresh from its paint order and kept boxes.
fn differing_drawn(kept: &Painted) -> Option<&'static str> {
    let drawn = crate::drawn_steps::DrawnSteps::of(&kept.order.steps, &kept.kept_boxes);
    (kept.drawn != drawn).then_some("drawn steps")
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

/// Each item a kept box holds, with its slot.
fn kept_items(kept: &Painted) -> impl Iterator<Item = (usize, &vapors_scene::Item)> {
    let boxes = kept.kept_boxes.iter().enumerate();
    boxes.flat_map(|(slot, kept)| {
        kept.iter()
            .flat_map(move |kept| kept.all_items().map(move |item| (slot, item)))
    })
}
