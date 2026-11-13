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
use vapors_renderer::{ImagePipeline, KeptRange, QuadPipeline, ShadowPipeline, Target};
use vapors_scene::{Item, Rect, Run};
use vapors_text::{BlendParams, GlyphAtlas, ICON_BLEND, TextPipeline};

use super::ScenePainter;
use super::errors::PaintError;

#[path = "painter_kept_gpu.rs"]
mod gpu;
pub(super) use gpu::KeptGpu;

/// Which pipeline's buffer holds a batch's bytes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Kind {
    Shadows,
    Quads,
    /// Image and surface quads: the image pipeline's.
    Images,
}

/// The pipelines whose buffers hold kept bytes, and the atlas the text
/// pipeline's name.
pub(super) struct Pipelines<'a> {
    pub(super) shadows: &'a mut ShadowPipeline,
    pub(super) quads: &'a mut QuadPipeline,
    pub(super) images: &'a mut ImagePipeline,
    pub(super) text: &'a mut TextPipeline,
    pub(super) atlas: &'a mut GlyphAtlas,
}

impl ScenePainter {
    /// Draws `run`, a shadow, quad, image or surface batch keyed `key`,
    /// from its kept bytes moved by `shift`, writing them first when none
    /// are kept. Nothing is drawn when the write is refused (the refusal
    /// is on the target's encoder).
    pub(super) fn draw_retained(
        &mut self,
        target: &Target<'_>,
        painted: &Painted,
        (run, key, at): (&Run, BatchKey, usize),
        shift: [f32; 2],
    ) -> Result<(), PaintError> {
        let pipelines = Pipelines {
            shadows: &mut self.shadows,
            quads: &mut self.quads,
            images: &mut self.images,
            text: &mut self.text,
            atlas: &mut self.atlas,
        };
        let Some(range) = self.kept.range(pipelines, target, (run, key, at)) else {
            return Ok(());
        };
        match run {
            Run::Shadows(_) => self.shadows.draw_kept(target, range, shift),
            Run::Quads(_) => self.quads.draw_kept(target, range, shift),
            Run::Images(items) => {
                let images = &mut self.images;
                self.textures
                    .draw_kept(images, target, painted, (items, range, shift));
            }
            Run::Surfaces(items) => {
                let images = &mut self.images;
                self.surfaces
                    .draw_kept(images, target, painted, (items, range, shift))?;
            }
            Run::Texts(_) | Run::Icons(_) => unreachable!("Kind::of refused it in range"),
        }
        Ok(())
    }

    /// Draws `run`, a text or icon batch keyed `key`, from its kept
    /// bytes moved by `shift`, building and writing them first when none are current
    /// (`painter_kept_text.rs`). Nothing is drawn for a batch with no
    /// ink, or when the write is refused (the refusal is on the
    /// target's encoder).
    pub(super) fn draw_kept_text(
        &mut self,
        target: &Target<'_>,
        painted: &Painted,
        (run, key, at): (&Run, BatchKey, usize),
        shift: [f32; 2],
    ) -> Result<(), PaintError> {
        let parts = (&mut self.text, &mut self.atlas);
        let kept = self
            .kept
            .text_range(parts, (target, painted), (run, key, at))?;
        if let Some(kept) = kept {
            self.text
                .draw_kept(target, &mut self.atlas, kept, blend(run), shift);
        }
        Ok(())
    }

    /// Releases the bytes of every batch the frame did not draw.
    ///
    /// # Panics
    ///
    /// When the pipelines' kept buffers then hold other than the bytes
    /// of the batches kept: a kept range was dropped unreleased.
    pub(super) fn release_undrawn(&mut self) {
        let mut pipelines = Pipelines {
            shadows: &mut self.shadows,
            quads: &mut self.quads,
            images: &mut self.images,
            text: &mut self.text,
            atlas: &mut self.atlas,
        };
        self.kept.release_undrawn(&mut pipelines);
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
        self.shadows.retained_bytes() + self.quads.retained_bytes() + images + text
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
            Kind::Images => self.images.keep(target, bytes),
        }
    }

    fn release(&mut self, kind: Kind, range: KeptRange) {
        match kind {
            Kind::Shadows => self.shadows.release(range),
            Kind::Quads => self.quads.release(range),
            Kind::Images => self.images.release(range),
        }
    }
}

impl Kind {
    /// The pipeline that keeps `run`'s bytes; `None` for text and icon
    /// runs, kept by the text pipeline (`painter_kept_text.rs`).
    pub(super) fn of_run(run: &Run) -> Option<Self> {
        match run {
            Run::Shadows(_) => Some(Self::Shadows),
            Run::Quads(_) => Some(Self::Quads),
            Run::Images(_) | Run::Surfaces(_) => Some(Self::Images),
            Run::Texts(_) | Run::Icons(_) => None,
        }
    }

    fn of(run: &Run) -> Self {
        Self::of_run(run).expect("only shadow, quad, image and surface runs are kept")
    }
}

/// `run`'s instance or vertex bytes, as its pipeline draws them.
pub(super) fn run_bytes(run: &Run) -> Vec<u8> {
    match run {
        Run::Shadows(shadows) => ShadowPipeline::bytes(shadows),
        Run::Quads(quads) => QuadPipeline::bytes(quads),
        Run::Images(items) | Run::Surfaces(items) => {
            ImagePipeline::rect_bytes(items.iter().map(item_rect))
        }
        Run::Texts(_) | Run::Icons(_) => unreachable!("text and icon runs are not kept"),
    }
}

/// The blend a text or icon run draws with.
fn blend(run: &Run) -> BlendParams {
    match run {
        Run::Icons(_) => ICON_BLEND,
        _ => BlendParams::default(),
    }
}

/// An image or surface item's rect.
fn item_rect(item: &Item) -> Rect {
    match item {
        Item::Image { rect, .. } | Item::Surface { rect, .. } => *rect,
        other => unreachable!("an image or surface run holds {other:?}"),
    }
}
