//! Drawing an update: clear to the canvas colour (or, drawing over what
//! the target holds, not), then each run of the scene's items with the
//! pipeline for its kind, in painting order, under the run's clip as a
//! scissor; a run clipped to nothing is not drawn, though its surfaces
//! are refused as they would be in view.

use vapors_elements::Painted;
use vapors_renderer::{ImagePipeline, QuadPipeline, ShadowPipeline, Target, clear_target};
use vapors_scene::{Rect, Run, Scissor, Size};
use vapors_text::{AtlasConfig, GlyphAtlas, TextPipeline};

/// The pipelines and glyph atlas for one target format.
pub struct ScenePainter {
    shadows: ShadowPipeline,
    quads: QuadPipeline,
    text: TextPipeline,
    atlas: GlyphAtlas,
    images: ImagePipeline,
    textures: Textures,
    surfaces: Surfaces,
    counts: FrameCounts,
}

impl ScenePainter {
    /// Pipelines for targets of `format`: `Rgba8Unorm` or `Bgra8Unorm`,
    /// the formats its shaders write sRGB-encoded values for (as the
    /// platform's surfaces are).
    ///
    /// # Errors
    ///
    /// [`UnsupportedFormat`] for any other format, whose colours would
    /// come out wrong: an sRGB format encodes twice, a float format
    /// holds encoded values as linear ones.
    pub fn new(
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
    ) -> Result<Self, UnsupportedFormat> {
        if !SUPPORTED.contains(&format) {
            return Err(UnsupportedFormat(format));
        }
        let atlas = GlyphAtlas::new(AtlasConfig::default());
        Ok(Self {
            shadows: ShadowPipeline::new(device, format),
            quads: QuadPipeline::new(device, format),
            text: TextPipeline::new(device, format, &atlas),
            atlas,
            images: ImagePipeline::new(device, format),
            textures: Textures::default(),
            surfaces: Surfaces::default(),
            counts: FrameCounts::default(),
        })
    }

    /// Clears `target` to the canvas colour and draws the scene over it.
    ///
    /// # Errors
    ///
    /// [`PaintError`] when a glyph or a `gpu-surface` cannot be drawn;
    /// the frame is then incomplete.
    pub fn draw(&mut self, target: &Target<'_>, painted: &Painted) -> Result<(), PaintError> {
        clear_target(target, painted.canvas());
        self.paint(target, painted)
    }

    /// Draws the scene over what `target` already holds, with no clear
    /// and no canvas colour: where no element paints, the target is
    /// left as it was. For a document over an app's own drawing.
    ///
    /// # Errors
    ///
    /// [`DrawOverError::CanvasBackground`], drawing nothing, when `html`
    /// or `body` sets a background: it is the canvas's, which drawing
    /// over never paints. As [`ScenePainter::draw`] otherwise.
    pub fn draw_over(
        &mut self,
        target: &Target<'_>,
        painted: &Painted,
    ) -> Result<(), DrawOverError> {
        if let Some(from) = painted.canvas_from() {
            return Err(DrawOverError::CanvasBackground(from));
        }
        self.paint(target, painted).map_err(DrawOverError::Paint)
    }

    /// Each run of the scene, in order, under its clip. A refused frame
    /// still ends the surfaces' frame: those it did not paint are inactive.
    fn paint(&mut self, target: &Target<'_>, painted: &Painted) -> Result<(), PaintError> {
        self.begin_counts();
        self.atlas.begin_frame();
        self.textures.begin_frame();
        self.surfaces.begin_frame();
        let runs = self.paint_runs(target, painted);
        self.surfaces.end_frame();
        runs?;
        self.textures.end_frame();
        Ok(())
    }

    fn paint_runs(&mut self, target: &Target<'_>, painted: &Painted) -> Result<(), PaintError> {
        for batch in painted.scene().batches() {
            match clipped(target, batch.clip) {
                Some(clipped) => self.draw_run(&clipped, painted, &batch.run)?,
                None => self
                    .surfaces
                    .check_clipped(target.device, painted, &batch.run)?,
            }
        }
        Ok(())
    }

    /// Draws `run` with the pipeline for its kind under `target`'s
    /// scissor, and counts the work it issued. A draw the pipeline's
    /// buffers refused ends the frame here.
    fn draw_run(
        &mut self,
        target: &Target<'_>,
        painted: &Painted,
        run: &Run<'_>,
    ) -> Result<(), PaintError> {
        let result = self.issue_run(target, painted, run);
        self.count_run(run);
        result?;
        target
            .encoder
            .take_refusal()
            .map_or(Ok(()), |refused| Err(refused.into()))
    }

    /// Issues `run`'s writes and draws; a refusal leaves those already
    /// issued for the caller to count.
    fn issue_run(
        &mut self,
        target: &Target<'_>,
        painted: &Painted,
        run: &Run<'_>,
    ) -> Result<(), PaintError> {
        match *run {
            Run::Shadows(ref shadows) => self.shadows.draw(target, shadows),
            Run::Quads(ref quads) => self.quads.draw(target, quads),
            Run::Texts(items) => self.draw_texts(target, painted, items)?,
            Run::Icons(items) => self.draw_icons(target, painted, items)?,
            Run::Images(items) => self.textures.draw(&mut self.images, target, painted, items),
            Run::Surfaces(items) => self
                .surfaces
                .draw(&mut self.images, target, painted, items)?,
        }
        Ok(())
    }
}

/// Where a batch may draw.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Region {
    /// The whole target.
    Whole,
    /// Inside this scissor.
    Within(Scissor),
    /// Nowhere: the batch is not drawn.
    Nowhere,
}

/// Where a batch under `clip` may draw on a `viewport`-sized target
/// that has its own scissor `outer`: inside both.
fn region(outer: Option<Scissor>, clip: Option<Rect>, viewport: Size) -> Region {
    let inner = match clip.map(|clip| clip.scissor(viewport)) {
        None => return outer.map_or(Region::Whole, Region::Within),
        Some(None) => return Region::Nowhere,
        Some(Some(inner)) => inner,
    };
    match outer.map_or(Some(inner), |outer| outer.intersect(inner)) {
        Some(both) => Region::Within(both),
        None => Region::Nowhere,
    }
}

/// `target` cut to `clip`; `None` when nothing of it is left.
fn clipped<'a>(target: &Target<'a>, clip: Option<Rect>) -> Option<Target<'a>> {
    let scissor = match region(target.scissor, clip, target.viewport) {
        Region::Whole => None,
        Region::Within(scissor) => Some(scissor),
        Region::Nowhere => return None,
    };
    Some(Target { scissor, ..*target })
}

#[path = "painter_counts.rs"]
mod counts;
pub use counts::FrameCounts;
#[path = "painter_images.rs"]
mod images;
#[path = "painter_text.rs"]
mod text;
#[path = "painter_tooltip.rs"]
mod tooltip;
use images::Textures;
#[path = "painter_errors.rs"]
mod errors;
pub use errors::{DrawOverError, MountError, PaintError, SUPPORTED, UnsupportedFormat};
#[path = "painter_surface_state.rs"]
mod surface_state;
pub use surface_state::SurfaceState;
#[path = "painter_surfaces.rs"]
mod surfaces;
use surfaces::Surfaces;
// For the root `gpu_` test module (the tier split needs a gpu_ id).
#[cfg(test)]
pub(crate) use surfaces::{SCOPE_DEADLINE, wants_frame_around_a_pending_scope};

#[cfg(test)]
#[path = "painter_tests.rs"]
mod tests;
