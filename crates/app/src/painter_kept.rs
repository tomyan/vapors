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

#[path = "painter_kept_entries.rs"]
mod entries;
#[path = "painter_kept_items.rs"]
mod items;
pub(super) use bytes::{Kind, RunBytes, rows_fit, rows_in_range, rows_needed, run_bytes};
use items::{ItemParts, draw_items, draw_meshes, item_place};

/// The painter's [`Pipelines`], each borrowed from its own field, so the
/// painter's other fields stay free to borrow beside them.
macro_rules! pipelines {
    ($painter:expr) => {
        Pipelines {
            shadows: &mut $painter.shadows,
            quads: &mut $painter.quads,
            closed_forms: &mut $painter.closed_forms,
            meshes: &mut $painter.meshes,
            shapes: &$painter.shapes,
            images: &mut $painter.images,
            text: &mut $painter.text,
            atlas: &mut $painter.atlas,
        }
    };
}

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
        let Some((frame, clips, rows_base)) = self.table_entries(target, run, moved) else {
            return Ok(());
        };
        let drawn = self.draw_retained_at(
            (target, painted),
            (run, key, at),
            moved,
            (frame, &clips, rows_base),
        );
        self.clips.runs = clips; // the scratch back, for the next batch
        drawn
    }

    /// [`draw_retained`](Self::draw_retained) at the table entries
    /// worked out for it.
    fn draw_retained_at(
        &mut self,
        (target, painted): (&Target<'_>, &Painted),
        (run, key, at): (&Run, BatchKey, usize),
        moved: &Moved<'_>,
        (frame, clips, rows_base): (u32, &[(u32, u32)], u32),
    ) -> Result<(), PaintError> {
        let rows = moved.rows.items();
        // image and surface bytes name no entry: their draw's uniform does
        let named = if matches!(run, Run::Images(_) | Run::Surfaces(_)) {
            (0, &[][..])
        } else {
            (frame, clips)
        };
        let pipelines = pipelines!(self);
        let Some(kept) = self.kept.range(
            pipelines,
            target,
            (run, (named, rows, moved.offsets.len()), key, at),
        )?
        else {
            return Ok(());
        };
        let range = kept.range;
        let table = (&mut self.transforms.table, &mut self.clips.table, rows_base);
        match run {
            Run::Shadows(_) => self.shadows.draw_kept(target, range, table),
            Run::Quads(_) => self.quads.draw_kept(target, range, table),
            Run::Shapes(_) => self.closed_forms.draw_kept(target, range, table),
            Run::Meshes(_) => {
                let shapes = (&self.shapes, &mut self.mesh_entries);
                let (transforms, _no_clips, rows_base) = table;
                draw_meshes(
                    &mut self.meshes,
                    shapes,
                    target,
                    (range, kept.meshes, (transforms, rows_base)),
                )?;
            }
            Run::Images(_) | Run::Surfaces(_) => {
                let parts = ItemParts {
                    images: &mut self.images,
                    textures: &mut self.textures,
                    surfaces: &mut self.surfaces,
                    place: item_place(
                        (&mut self.transforms, &mut self.clips.table),
                        (frame, rows_base),
                        clips,
                    ),
                };
                draw_items(parts, (target, painted), (run, range, moved))?;
            }
            Run::Texts(_) | Run::Icons(_) => unreachable!("Kind::of refused it in range"),
        }
        Ok(())
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
        let Some((frame, clips, rows_base)) = self.table_entries(target, run, moved) else {
            return Ok(());
        };
        let parts = (&mut self.text, &mut self.atlas);
        let rows = (
            (frame, clips.as_slice()),
            moved.rows.items(),
            moved.offsets.len(),
        );
        let kept = self
            .kept
            .text_range(parts, (target, painted), (run, rows, key, at))?;
        if let Some(kept) = kept {
            let tables = (&mut self.transforms.table, &mut self.clips.table);
            let params = (blend(run), tables, rows_base);
            self.text.draw_kept(target, &mut self.atlas, kept, params);
        }
        self.clips.runs = clips; // the scratch back, for the next batch
        Ok(())
    }

    /// Releases the bytes of every batch the frame did not draw.
    ///
    /// # Panics
    ///
    /// When the pipelines' kept buffers then hold other than the bytes
    /// of the batches kept: a kept range was dropped unreleased.
    pub(super) fn release_undrawn(&mut self, queue: &wgpu::Queue) {
        let mut pipelines = pipelines!(self);
        self.kept.release_undrawn(&mut pipelines);
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

/// The blend a text or icon run draws with.
fn blend(run: &Run) -> BlendParams {
    match run {
        Run::Icons(_) => ICON_BLEND,
        _ => BlendParams::default(),
    }
}

#[cfg(test)]
#[path = "painter_kept_rows_fit_tests.rs"]
mod rows_fit_tests;
