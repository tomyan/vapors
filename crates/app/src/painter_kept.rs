//! The painter's batches kept on the GPU (IU4 PR 2 and 3,
//! `docs/plans/iu4.md` section 2): each shadow, quad, image and surface
//! batch's instance or vertex bytes (text and icon batches':
//! `painter_kept_text.rs`), written into its pipeline's kept buffer on the first
//! draw of its key (`vapors_elements::BatchKey`) and drawn from there
//! while the key lasts, on targets of any viewport, moved by the shift
//! of the scroll frame they are in. A key a frame does not draw has its
//! bytes released after the frame. Each `draw` or `draw_over` call is a
//! frame, so a painter keeps the bytes of one document: a second drawn
//! by the same painter releases the first's, and both write every batch
//! on every call. Draw each document with a painter of its own.
//! After each frame the pipelines' kept buffers must hold exactly the
//! kept batches' bytes; a range dropped unreleased panics there.
//!
//! A debug build builds every kept batch's bytes afresh on each draw
//! and compares them with the bytes kept (`painter_kept_check.rs`).

use vapors_elements::{BatchKey, Painted};
use vapors_renderer::{
    ClosedFormPipeline, ImagePipeline, KeptRange, MeshPipeline, QuadPipeline, ShadowPipeline,
    Target,
};
use vapors_scene::Run;
use vapors_text::{BlendParams, GlyphAtlas, ICON_BLEND, TextPipeline};

use super::ScenePainter;
use super::errors::PaintError;
use super::rows::Moved;
use crate::shape_store::ShapeStore;

#[path = "painter_kept_gpu.rs"]
mod gpu;
pub(super) use gpu::KeptGpu;
#[path = "painter_kept_bytes.rs"]
mod bytes;
pub(super) use bytes::{Kind, RunBytes, rows_in_range, run_bytes};

/// The pipelines whose buffers hold kept bytes, the atlas the text
/// pipeline's name, and the store shape items name entries in.
pub(super) struct Pipelines<'a> {
    pub(super) shadows: &'a mut ShadowPipeline,
    pub(super) quads: &'a mut QuadPipeline,
    pub(super) closed_forms: &'a mut ClosedFormPipeline,
    pub(super) meshes: &'a mut MeshPipeline,
    pub(super) shapes: &'a ShapeStore,
    pub(super) images: &'a mut ImagePipeline,
    pub(super) text: &'a mut TextPipeline,
    pub(super) atlas: &'a mut GlyphAtlas,
}

impl ScenePainter {
    /// Draws `run`, a shadow, quad, shape, image or surface batch keyed
    /// `key`, from its kept bytes moved by `shift`, writing them first
    /// when none are kept. A shape run naming an entry the store cannot
    /// draw is refused when its bytes are built, before any is written
    /// (the store only grows, so kept bytes stay drawable). Nothing is
    /// drawn when the write is refused (the refusal is on the target's
    /// encoder).
    pub(super) fn draw_retained(
        &mut self,
        target: &Target<'_>,
        painted: &Painted,
        (run, key, at): (&Run, BatchKey, usize),
        moved: &Moved<'_>,
    ) -> Result<(), PaintError> {
        let rows = moved.rows.items();
        let Some((frame, clip, rows_base)) = self.table_entries(target, (run, key), moved) else {
            return Ok(());
        };
        let pipelines = Pipelines {
            shadows: &mut self.shadows,
            quads: &mut self.quads,
            closed_forms: &mut self.closed_forms,
            meshes: &mut self.meshes,
            shapes: &self.shapes,
            images: &mut self.images,
            text: &mut self.text,
            atlas: &mut self.atlas,
        };
        let Some(kept) = self.kept.range(
            pipelines,
            target,
            (run, ((frame, clip), rows, moved.offsets.len()), key, at),
        )?
        else {
            return Ok(());
        };
        let (range, shift) = (kept.range, moved.shift);
        let table = (&mut self.transforms.table, &mut self.clips.table, rows_base);
        match run {
            Run::Shadows(_) => self.shadows.draw_kept(target, range, table),
            Run::Quads(_) => self.quads.draw_kept(target, range, table),
            Run::Shapes(_) => self.closed_forms.draw_kept(target, range, table),
            Run::Meshes(_) => {
                let shift = unrowed(shift, rows);
                let drawn = self
                    .shapes
                    .uploaded(kept.meshes)
                    .map_err(PaintError::Shape)?;
                let entries = (&mut self.mesh_entries, drawn.as_slice());
                self.meshes.draw_kept(target, range, shift, entries);
            }
            Run::Images(items) => {
                let images = &mut self.images;
                self.textures
                    .draw_kept(images, target, painted, (items, range, moved));
            }
            Run::Surfaces(items) => {
                let images = &mut self.images;
                self.surfaces
                    .draw_kept(images, target, painted, (items, range, moved))?;
            }
            Run::Texts(_) | Run::Icons(_) => unreachable!("Kind::of refused it in range"),
        }
        Ok(())
    }

    /// For the runs whose items name entries of the transform and clip
    /// tables (shadows, quads, shapes): their frame's entry, set to its
    /// shift, their clip's entry (0 for none, `painter_clips.rs`), and the
    /// first of batch `key`'s rows' entries (0 for a batch in one frame);
    /// zeros for the others. `None` when a table could not grow (the
    /// refusal is on the target's encoder).
    fn table_entries(
        &mut self,
        target: &Target<'_>,
        (run, key): (&Run, BatchKey),
        moved: &Moved<'_>,
    ) -> Option<(u32, u32, u32)> {
        match Kind::of_run(run) {
            Some(Kind::Shadows | Kind::Quads | Kind::ClosedForms) => {
                let frame = self
                    .transforms
                    .frame_entry(target, moved.frame, moved.shift)?;
                let clip = match moved.clip {
                    Some(clip) => self.clip_entry(target, (frame, clip), moved)?,
                    None => 0,
                };
                let rows_base = self.transforms.rows_base(target, key, moved.offsets)?;
                Some((frame, clip, rows_base))
            }
            _ => Some((0, 0, 0)),
        }
    }

    /// The entry of `clip` in `moved`'s frame, whose transform entry is
    /// `frame`, checked against the scissor path's placing of it.
    fn clip_entry(
        &mut self,
        target: &Target<'_>,
        (frame, clip): (u32, vapors_scene::Rect),
        moved: &Moved<'_>,
    ) -> Option<u32> {
        let at = self.clips.entry(target, (moved.frame, frame), clip)?;
        let tables = (&self.transforms.table, &self.clips.table);
        super::clips::assert_agrees(tables, at, (clip, moved.draw));
        Some(at)
    }

    /// Draws `run`, a text or icon batch keyed `key`, from its kept
    /// bytes moved by `shift`, building and writing them first when none
    /// are current (`painter_kept_text.rs`). Nothing is drawn for a batch with no
    /// ink, or when the write is refused (the refusal is on the
    /// target's encoder).
    pub(super) fn draw_kept_text(
        &mut self,
        target: &Target<'_>,
        painted: &Painted,
        (run, key, at): (&Run, BatchKey, usize),
        moved: &Moved<'_>,
    ) -> Result<(), PaintError> {
        let parts = (&mut self.text, &mut self.atlas);
        let rowed = (run, moved.rows.items(), key, at);
        let kept = self.kept.text_range(parts, (target, painted), rowed)?;
        if let Some(kept) = kept {
            let at = (moved.shift, moved.table);
            self.text
                .draw_kept_in_rows(target, &mut self.atlas, kept, (blend(run), at));
        }
        Ok(())
    }

    /// Releases the bytes of every batch the frame did not draw.
    ///
    /// # Panics
    ///
    /// When the pipelines' kept buffers then hold other than the bytes
    /// of the batches kept: a kept range was dropped unreleased.
    pub(super) fn release_undrawn(&mut self, queue: &wgpu::Queue) {
        let mut pipelines = Pipelines {
            shadows: &mut self.shadows,
            quads: &mut self.quads,
            closed_forms: &mut self.closed_forms,
            meshes: &mut self.meshes,
            shapes: &self.shapes,
            images: &mut self.images,
            text: &mut self.text,
            atlas: &mut self.atlas,
        };
        self.kept.release_undrawn(&mut pipelines);
        self.row_tables.release_undrawn();
        // clips first, then frames (flat-scene-f3.md item 4)
        let clips = self.clips.release_undrawn(queue);
        self.kept.assert_none_names_clip(&clips);
        let frames = self.transforms.release_undrawn();
        self.clips.assert_none_names(&frames);
        let (held, retained) = (self.kept.held_bytes(), self.retained_bytes());
        assert_eq!(
            held, retained,
            "the kept batches hold {held} bytes, the pipelines' kept buffers {retained}: a \
             KeptRange was dropped without release (painter_kept_gpu.rs); reproduce with \
             `cargo xtask test`"
        );
    }

    /// Bytes held in the pipelines' kept buffers.
    pub(super) fn retained_bytes(&self) -> u64 {
        let (images, text) = (self.images.retained_bytes(), self.text.retained_bytes());
        let (shadows, quads) = (self.shadows.retained_bytes(), self.quads.retained_bytes());
        let shapes = self.closed_forms.retained_bytes() + self.meshes.retained_bytes();
        shadows + quads + shapes + images + text
    }
}

/// Gives the kept text pins back, which debug builds require of every
/// `AtlasPins` (`vapors_text::AtlasPins`).
impl Drop for ScenePainter {
    fn drop(&mut self) {
        self.kept.release_texts(&mut self.text, &mut self.atlas);
    }
}

impl Pipelines<'_> {
    fn keep(&mut self, kind: Kind, target: &Target<'_>, bytes: &[u8]) -> Option<KeptRange> {
        match kind {
            Kind::Shadows => self.shadows.keep(target, bytes),
            Kind::Quads => self.quads.keep(target, bytes),
            Kind::ClosedForms => self.closed_forms.keep(target, bytes),
            Kind::Meshes => self.meshes.keep(target, bytes),
            Kind::Images => self.images.keep(target, bytes),
        }
    }

    fn release(&mut self, kind: Kind, range: KeptRange) {
        match kind {
            Kind::Shadows => self.shadows.release(range),
            Kind::Quads => self.quads.release(range),
            Kind::ClosedForms => self.closed_forms.release(range),
            Kind::Meshes => self.meshes.release(range),
            Kind::Images => self.images.release(range),
        }
    }
}

/// `shift`, for a mesh batch, which is batched in one frame
/// (vapors-elements `framed_batches.rs` `resolved`): drawn at that
/// frame's shift, with no rows (meshes move to the transform table in
/// flat-scene F2.3).
///
/// # Panics
///
/// When the batch has rows: the mesh pipeline has no table of row
/// offsets, so every row would draw at the first's place.
fn unrowed(shift: vapors_renderer::Shift, rows: &[u32]) -> vapors_renderer::Shift {
    assert!(
        rows.is_empty(),
        "a mesh batch spans {} items' rows, and its pipeline takes no table of row \
         offsets: shapes and meshes in a lane row are batched in the row's frame \
         (vapors-elements framed_batches.rs `resolved`); reproduce with `cargo xtask test`",
        rows.len()
    );
    shift
}

/// The blend a text or icon run draws with.
fn blend(run: &Run) -> BlendParams {
    match run {
        Run::Icons(_) => ICON_BLEND,
        _ => BlendParams::default(),
    }
}
