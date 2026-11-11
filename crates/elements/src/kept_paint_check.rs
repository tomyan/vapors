//! The debug build's check on a kept paint (`kept_paint.rs`): the
//! update paints again and compares every part of the two paints but
//! `from`, which the update sets after, the kept boxes, and the counts.

use std::rc::Rc;

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
// Exact: a paint made from the same inputs has the same scale, bit for bit.
#[allow(clippy::float_cmp)]
pub(crate) fn assert_same(kept: &Painted, fresh: &Painted) {
    let mut parts = differing!(kept, fresh; canvas, canvas_from, scale, scene, rects,
        paddings, visible, hits, cursors, images, icons, surfaces, scroll_chains, ranges,
        text_origins);
    if !same_layouts(&kept.texts, &fresh.texts) {
        parts.push("texts");
    }
    assert!(
        parts.is_empty(),
        "the kept paint differs from a new paint in {parts:?}: a paint input is not \
         compared in PaintedFrom (crates/elements/src/kept_paint.rs) or BoxFrom \
         (crates/elements/src/kept_box.rs), or a copy renumbers wrongly \
         (paint_partial.rs); reproduce with `cargo xtask test`"
    );
}

/// Whether each text run is the same layout: a layout kept in the text
/// cache is shared, not copied, by every paint that uses it.
fn same_layouts(kept: &[Rc<vapors_text::Layout>], fresh: &[Rc<vapors_text::Layout>]) -> bool {
    kept.len() == fresh.len() && kept.iter().zip(fresh).all(|(a, b)| Rc::ptr_eq(a, b))
}
