//! Whether a batch's run is the same as an earlier one's to the bit
//! (`kept_batches_keys.rs`). `==` on runs compares their floats, so
//! -0.0 equals 0.0, while the bytes a painter keeps for a shadow or quad
//! run hold each float's bits: a run equal under `==` whose zeros differ
//! in sign would draw kept bytes that differ from its own.

use vapors_scene::{Color, Item, Quad, Rect, Run, Shadow, Shape};

/// `was` and `now` are equal, and every float of a shadow, quad, shape,
/// image or surface in them has the same bits.
pub(crate) fn same_bits(was: &Run, now: &Run) -> bool {
    was == now && floats_same_bits(was, now)
}

/// The floats of the items of `was` and `now`, runs of one kind and
/// length, have the same bits; text and icon runs, which are not kept,
/// always do.
fn floats_same_bits(was: &Run, now: &Run) -> bool {
    match (was, now) {
        (Run::Shadows(was), Run::Shadows(now)) => pairs_same(was, now, shadow_floats),
        (Run::Quads(was), Run::Quads(now)) => pairs_same(was, now, quad_floats),
        (Run::Shapes(was), Run::Shapes(now)) | (Run::Meshes(was), Run::Meshes(now)) => {
            pairs_same(was, now, shape_floats)
        }
        (Run::Images(was), Run::Images(now)) | (Run::Surfaces(was), Run::Surfaces(now)) => {
            pairs_same(was, now, item_floats)
        }
        _ => true,
    }
}

/// Each pair of `was` and `now`'s items has floats of the same bits.
fn pairs_same<T, const N: usize>(was: &[T], now: &[T], floats: fn(&T) -> [[f32; 4]; N]) -> bool {
    let same = |(was, now): (&T, &T)| {
        let (was, now) = (floats(was), floats(now));
        let mut pairs = was.as_flattened().iter().zip(now.as_flattened());
        pairs.all(|(was, now)| was.to_bits() == now.to_bits())
    };
    was.iter().zip(now).all(same)
}

/// Every float of `quad`: a field added to `Quad` stops this compiling.
fn quad_floats(quad: &Quad) -> [[f32; 4]; 8] {
    let Quad {
        rect,
        background,
        border,
        border_colors,
        radii,
    } = *quad;
    let [top, right, bottom, left] = border_colors.map(color_floats);
    [
        rect_floats(rect),
        color_floats(background),
        border,
        top,
        right,
        bottom,
        left,
        radii,
    ]
}

/// Every float of `shadow`: a field added to `Shadow` stops this
/// compiling.
fn shadow_floats(shadow: &Shadow) -> [[f32; 4]; 6] {
    let Shadow {
        rect,
        radii,
        blur,
        color,
        caster,
        caster_radii,
    } = *shadow;
    let (rect, color, caster) = (rect_floats(rect), color_floats(color), rect_floats(caster));
    [rect, radii, color, caster, caster_radii, [blur; 4]]
}

/// Every float of `shape`: a field added to `Shape` stops this
/// compiling.
fn shape_floats(shape: &Shape) -> [[f32; 4]; 4] {
    let Shape {
        shape: _,
        kind: _,
        transform,
        colour,
        bounds,
    } = *shape;
    let [linear @ .., e, f] = transform;
    [
        linear,
        [e, f, 0.0, 0.0],
        color_floats(colour),
        rect_floats(bounds),
    ]
}

/// An image or surface item's rect.
fn item_floats(item: &Item) -> [[f32; 4]; 1] {
    match *item {
        Item::Image { rect, .. } | Item::Surface { rect, .. } => [rect_floats(rect)],
        ref other => unreachable!("an image or surface run holds {other:?}"),
    }
}

fn rect_floats(
    Rect {
        left,
        top,
        right,
        bottom,
    }: Rect,
) -> [f32; 4] {
    [left, top, right, bottom]
}

fn color_floats(Color { r, g, b, a }: Color) -> [f32; 4] {
    [r, g, b, a]
}

#[cfg(test)]
#[path = "run_bits_tests.rs"]
mod tests;
