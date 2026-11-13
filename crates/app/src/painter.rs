//! Drawing an update: clear to the canvas colour (or, drawing over what
//! the target holds, not), then each run of the scene's items with the
//! pipeline for its kind, in painting order, moved by its scroll frame's
//! shift, under the run's clip moved the same way and cut to the frame's
//! clip as a scissor. Items that paint nothing inside the target under their clip
//! are culled when the scene is batched; a run clipped to nothing is not
//! drawn, though its surfaces are refused as they would be in view.

use vapors_elements::Painted;
use vapors_renderer::{ImagePipeline, QuadPipeline, ShadowPipeline, Target, clear_target};
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
    kept: KeptGpu,
    counts: FrameCounts,
}

impl ScenePainter {
    /// Pipelines for targets of `format`: `Rgba8Unorm` or `Bgra8Unorm`,
    /// the formats its shaders write sRGB-encoded values for (as the
    /// platform's surfaces are).
    ///
    /// The glyph atlas is sized for the device:
    /// `AtlasConfig::for_max_texture_side` of its
    /// `max_texture_dimension_2d`.
    ///
    /// # Errors
    ///
    /// [`NewPainterError::Format`] for any other format, whose colours
    /// would come out wrong: an sRGB format encodes twice, a float format
    /// holds encoded values as linear ones. [`NewPainterError::Atlas`]
    /// when the device's side is zero.
    pub fn new(
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
    ) -> Result<Self, NewPainterError> {
        let side = device.limits().max_texture_dimension_2d;
        Self::with_atlas(device, format, AtlasConfig::for_max_texture_side(side))
    }

    /// As [`new`](Self::new), with the glyph atlas sized by `atlas`.
    ///
    /// # Errors
    ///
    /// As [`new`](Self::new); [`NewPainterError::Atlas`] when the device
    /// cannot hold `atlas`'s pages (`AtlasConfig::checked`).
    pub fn with_atlas(
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
        atlas: AtlasConfig,
    ) -> Result<Self, NewPainterError> {
        if !SUPPORTED.contains(&format) {
            return Err(UnsupportedFormat(format).into());
        }
        let atlas = GlyphAtlas::new(atlas.checked(device.limits().max_texture_dimension_2d)?);
        Ok(Self {
            shadows: ShadowPipeline::new(device, format),
            quads: QuadPipeline::new(device, format),
            text: TextPipeline::new(device, format, &atlas),
            atlas,
            images: ImagePipeline::new(device, format),
            textures: Textures::default(),
            surfaces: Surfaces::default(),
            kept: KeptGpu::default(),
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
    /// Each call is one frame for the bytes kept between frames
    /// (`painter_kept.rs`): draw a second document with a painter of its
    /// own, or each call writes every batch the other released.
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
    /// It drops no texture, and its counts hold the bind groups still kept.
    /// Every frame releases the kept bytes of the batches it did not draw.
    fn paint(&mut self, target: &Target<'_>, painted: &Painted) -> Result<(), PaintError> {
        self.begin_counts();
        self.atlas.begin_frame();
        self.textures.begin_frame();
        self.surfaces.begin_frame();
        self.kept.begin_frame();
        let runs = self.paint_runs(target, painted);
        self.surfaces.end_frame();
        if runs.is_ok() {
            self.textures.end_frame();
        }
        self.release_undrawn();
        self.end_counts();
        runs
    }
}

#[path = "painter_counts.rs"]
mod counts;
#[path = "painter_kept.rs"]
mod kept;
#[path = "painter_region.rs"]
mod region;
#[path = "painter_runs.rs"]
mod runs;
use kept::KeptGpu;
#[cfg(debug_assertions)]
#[path = "painter_kept_check.rs"]
mod kept_check;
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
pub use errors::{
    DrawOverError, MountError, NewPainterError, PaintError, SUPPORTED, UnsupportedFormat,
};
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
