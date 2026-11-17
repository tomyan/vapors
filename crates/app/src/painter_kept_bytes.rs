//! What a kept batch's bytes are: the pipeline whose buffer holds them,
//! and the bytes each run's pipeline draws (`painter_kept.rs`); a mesh
//! run's with its items' entries, built once beside its instances.

use vapors_renderer::{
    ClosedFormPipeline, ImagePipeline, MeshPipeline, QuadPipeline, ShadowPipeline,
};
use vapors_scene::{Item, Rect, Run};

use super::super::errors::PaintError;
use crate::shape_store::ShapeStore;

/// Which pipeline's buffer holds a batch's bytes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Kind {
    Shadows,
    Quads,
    /// Closed-form shapes: the closed-form pipeline's.
    ClosedForms,
    /// Mesh shapes' instances: the mesh pipeline's.
    Meshes,
    /// Image and surface quads: the image pipeline's.
    Images,
}

impl Kind {
    /// The pipeline that keeps `run`'s bytes; `None` for text and icon
    /// runs, kept by the text pipeline (`painter_kept_text.rs`).
    pub(crate) fn of_run(run: &Run) -> Option<Self> {
        match run {
            Run::Shadows(_) => Some(Self::Shadows),
            Run::Quads(_) => Some(Self::Quads),
            Run::Shapes(_) => Some(Self::ClosedForms),
            Run::Meshes(_) => Some(Self::Meshes),
            Run::Images(_) | Run::Surfaces(_) => Some(Self::Images),
            Run::Texts(_) | Run::Icons(_) => None,
        }
    }

    pub(crate) fn of(run: &Run) -> Self {
        Self::of_run(run).expect("only shadow, quad, shape, image and surface runs are kept")
    }
}

/// A run's bytes as its pipeline keeps them, and for a mesh run the
/// store's index of each drawn item's entry, beside its instance.
pub(crate) struct RunBytes {
    pub(crate) bytes: Vec<u8>,
    pub(crate) meshes: Vec<usize>,
}

/// `run`'s instance or vertex bytes, as its pipeline draws them, a
/// shape run's entries from `shapes`; shadow, quad and shape items each
/// in the transform table's entry `frame` and its row of `rows` (none for
/// a batch in one frame). Images and surfaces, drawn an item at a time,
/// carry their row in the shift; shapes and meshes are batched in their
/// row's own frame (vapors-elements `framed_batches.rs` `resolved`), so
/// their `rows` are empty.
///
/// # Errors
/// A shape run naming an entry `shapes` cannot draw.
pub(crate) fn run_bytes(
    (run, frame, rows): (&Run, u32, &[u32]),
    shapes: &ShapeStore,
) -> Result<RunBytes, PaintError> {
    if let Run::Meshes(items) = run {
        let (instances, meshes) = shapes.mesh_instances(items).map_err(PaintError::Shape)?;
        let bytes = MeshPipeline::bytes(&instances);
        return Ok(RunBytes { bytes, meshes });
    }
    let bytes = match run {
        Run::Shadows(shadows) => ShadowPipeline::bytes_in(shadows, frame, rows),
        Run::Quads(quads) => QuadPipeline::bytes_in(quads, frame, rows),
        Run::Shapes(items) => {
            let forms = shapes.closed_forms(items).map_err(PaintError::Shape)?;
            ClosedFormPipeline::bytes_in(&forms, frame, rows)
        }
        Run::Images(items) | Run::Surfaces(items) => {
            ImagePipeline::rect_bytes(items.iter().map(item_rect))
        }
        Run::Meshes(_) => unreachable!("built above"),
        Run::Texts(_) | Run::Icons(_) => unreachable!("text and icon runs are not kept"),
    };
    Ok(RunBytes {
        bytes,
        meshes: Vec::new(),
    })
}

/// An image or surface item's rect.
fn item_rect(item: &Item) -> Rect {
    match item {
        Item::Image { rect, .. } | Item::Surface { rect, .. } => *rect,
        other => unreachable!("an image or surface run holds {other:?}"),
    }
}

/// Refuses, naming batch `key` and the item, an item row of `rows` that
/// is not in the batch's own range of `range` rows. An item whose row
/// were past it would read the table's next entries: another batch's
/// rows, or a frame's shift (`painter_transforms.rs`).
///
/// # Panics
///
/// As said; an empty range (a batch in one frame) holds no row, so any
/// row index is past it.
pub(crate) fn rows_in_range(key: vapors_elements::BatchKey, rows: &[u32], range: usize) {
    let past = rows.iter().position(|&row| row as usize >= range);
    if let Some(at) = past {
        panic!(
            "item {at} of batch {key:?} names row {}, past the {range} rows of the batch's range \
             in the transform table: each item of a batch over a lane's rows names one of the \
             rows the batch's offsets give (vapors-elements framed_batches_rows.rs); reproduce \
             with `cargo xtask test`",
            rows[at]
        );
    }
}
