//! The painter's batches kept on the GPU (IU4 PR 2, `docs/plans/iu4.md`
//! section 2): each shadow, quad, image and surface batch's instance or
//! vertex bytes, written into its pipeline's kept buffer on the first
//! draw of its key (`vapors_elements::BatchKey`) and drawn from there
//! while the key lasts. Image and surface quads are in clip space, so
//! their bytes are written again on a target of another viewport (one
//! scissored to the same view). A key
//! a frame does not draw has its bytes released after the frame.
//!
//! A debug build builds every kept batch's bytes afresh on each draw
//! and compares them with the bytes kept (`painter_kept_check.rs`).

use vapors_elements::{BatchKey, Painted};
use vapors_renderer::{ImagePipeline, KeptRange, QuadPipeline, ShadowPipeline, Target};
use vapors_scene::{Item, Rect, Run, Size};

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

/// The pipelines whose buffers hold kept bytes.
pub(super) struct Pipelines<'a> {
    pub(super) shadows: &'a mut ShadowPipeline,
    pub(super) quads: &'a mut QuadPipeline,
    pub(super) images: &'a mut ImagePipeline,
}

impl ScenePainter {
    /// Draws `run`, a shadow, quad, image or surface batch keyed `key`,
    /// from its kept bytes, writing them first when none are kept for
    /// the target's viewport. Nothing is drawn when the write is refused
    /// (the refusal is on the target's encoder).
    pub(super) fn draw_retained(
        &mut self,
        target: &Target<'_>,
        painted: &Painted,
        (run, key, at): (&Run, BatchKey, usize),
    ) -> Result<(), PaintError> {
        let pipelines = Pipelines {
            shadows: &mut self.shadows,
            quads: &mut self.quads,
            images: &mut self.images,
        };
        let Some(range) = self.kept.range(pipelines, target, (run, key, at)) else {
            return Ok(());
        };
        match run {
            Run::Shadows(_) => self.shadows.draw_kept(target, range),
            Run::Quads(_) => self.quads.draw_kept(target, range),
            Run::Images(items) => {
                let images = &mut self.images;
                self.textures
                    .draw_kept(images, target, painted, (items, range));
            }
            Run::Surfaces(items) => {
                let images = &mut self.images;
                self.surfaces
                    .draw_kept(images, target, painted, (items, range))?;
            }
            Run::Texts(_) | Run::Icons(_) => unreachable!("Kind::of refused it in range"),
        }
        Ok(())
    }

    /// Releases the bytes of every batch the frame did not draw.
    pub(super) fn release_undrawn(&mut self) {
        let mut pipelines = Pipelines {
            shadows: &mut self.shadows,
            quads: &mut self.quads,
            images: &mut self.images,
        };
        self.kept.release_undrawn(&mut pipelines);
    }

    /// Bytes held in the pipelines' kept buffers.
    pub(super) fn retained_bytes(&self) -> u64 {
        let images = self.images.retained_bytes();
        self.shadows.retained_bytes() + self.quads.retained_bytes() + images
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
    /// runs, which are drawn as each frame builds them.
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

/// `run`'s instance or vertex bytes on a target of `viewport`, as its
/// pipeline draws them.
pub(super) fn run_bytes(run: &Run, viewport: Size) -> Vec<u8> {
    match run {
        Run::Shadows(shadows) => ShadowPipeline::bytes(shadows),
        Run::Quads(quads) => QuadPipeline::bytes(quads),
        Run::Images(items) | Run::Surfaces(items) => {
            ImagePipeline::rect_bytes(viewport, items.iter().map(item_rect))
        }
        Run::Texts(_) | Run::Icons(_) => unreachable!("text and icon runs are not kept"),
    }
}

/// An image or surface item's rect.
fn item_rect(item: &Item) -> Rect {
    match item {
        Item::Image { rect, .. } | Item::Surface { rect, .. } => *rect,
        other => unreachable!("an image or surface run holds {other:?}"),
    }
}
