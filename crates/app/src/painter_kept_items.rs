//! Drawing an image, surface or mesh batch from its kept bytes
//! (`painter_kept.rs`): each item placed by its frame, row and clip
//! entries.

use vapors_elements::Painted;
use vapors_renderer::{KeptRange, MeshPipeline, Target, TransformTable};
use vapors_scene::Run;

use crate::painter::errors::PaintError;
use crate::painter::rows::Moved;
use crate::painter::transforms::ItemPlace;
use crate::shape_store::ShapeStore;

/// What an image or surface batch is drawn with.
pub(super) struct ItemParts<'a> {
    pub(super) images: &'a mut vapors_renderer::ImagePipeline,
    pub(super) textures: &'a mut crate::painter::images::Textures,
    pub(super) surfaces: &'a mut crate::painter::surfaces::Surfaces,
    pub(super) place: ItemPlace<'a>,
}

/// Draws image or surface `run`'s items with `parts`, each over its quad
/// in `range`, at its frame, row and clip entries.
pub(super) fn draw_items(
    parts: ItemParts<'_>,
    (target, painted): (&Target<'_>, &Painted),
    (run, range, moved): (&Run, &vapors_renderer::KeptRange, &Moved<'_>),
) -> Result<(), PaintError> {
    let ItemParts {
        images,
        textures,
        surfaces,
        place,
    } = parts;
    match run {
        Run::Images(items) => {
            textures.draw_kept(images, target, painted, (items, range, moved, place));
            Ok(())
        }
        Run::Surfaces(items) => {
            surfaces.draw_kept(images, target, painted, (items, range, moved, place))
        }
        other => unreachable!("{other:?} is not an image or surface run"),
    }
}

/// Where an image or surface batch's items are drawn: `transforms`'
/// table, with their frame's entry and their rows' first, and the clip
/// table with their clip runs.
pub(super) fn item_place<'a>(
    (transforms, clips): (
        &'a mut crate::painter::transforms::Transforms,
        &'a mut vapors_renderer::ClipTable,
    ),
    (frame, rows_base): (u32, u32),
    clip_runs: &'a [(u32, u32)],
) -> crate::painter::transforms::ItemPlace<'a> {
    crate::painter::transforms::ItemPlace {
        table: &mut transforms.table,
        frame,
        rows_base,
        clips,
        clip_runs,
    }
}

/// Draws a mesh batch's instances `range` holds, its items' entries
/// `items` from `shapes`, placed through `table` from its rows' first.
///
/// # Errors
/// An item whose entry the store has not uploaded.
pub(super) fn draw_meshes(
    meshes: &mut MeshPipeline,
    (shapes, entries): (&ShapeStore, &mut vapors_renderer::MeshEntries),
    target: &Target<'_>,
    (range, items, table): (&KeptRange, &[usize], (&mut TransformTable, u32)),
) -> Result<(), PaintError> {
    let drawn = shapes.uploaded(items).map_err(PaintError::Shape)?;
    meshes.draw_kept(target, range, table, (entries, drawn.as_slice()));
    Ok(())
}
