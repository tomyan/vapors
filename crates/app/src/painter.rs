//! Drawing an update: clear to the canvas colour, then each run of the
//! scene's items with the pipeline for its kind, in painting order,
//! under the run's clip as a scissor; a run clipped to nothing is not
//! drawn.

use vapors_elements::Painted;
use vapors_renderer::{QuadPipeline, ShadowPipeline, Target, clear_view};
use vapors_scene::{Item, Rect, Run, Scissor, Size};
use vapors_text::{AtlasConfig, AtlasError, BlendParams, GlyphAtlas, RenderTarget, TextPipeline};

/// The pipelines and glyph atlas for one target format.
pub struct ScenePainter {
    shadows: ShadowPipeline,
    quads: QuadPipeline,
    text: TextPipeline,
    atlas: GlyphAtlas,
}

impl ScenePainter {
    /// Pipelines for targets of `format`.
    #[must_use]
    pub fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        let atlas = GlyphAtlas::new(AtlasConfig::default());
        Self {
            shadows: ShadowPipeline::new(device, format),
            quads: QuadPipeline::new(device, format),
            text: TextPipeline::new(device, format, &atlas),
            atlas,
        }
    }

    /// Clears `target` to the canvas colour and draws the scene over it.
    ///
    /// # Errors
    ///
    /// [`AtlasError`] when a glyph cannot be rasterised or a frame's
    /// glyphs do not fit the atlas page; the frame is then incomplete.
    pub fn draw(&mut self, target: &Target<'_>, painted: &Painted) -> Result<(), AtlasError> {
        clear_view(target.device, target.queue, target.view, painted.canvas());
        self.atlas.begin_frame();
        for batch in painted.scene().batches() {
            let Some(clipped) = clipped(target, batch.clip) else {
                continue;
            };
            match batch.run {
                Run::Shadows(shadows) => self.shadows.draw(&clipped, &shadows),
                Run::Quads(quads) => self.quads.draw(&clipped, &quads),
                Run::Texts(items) => self.draw_texts(&clipped, painted, items)?,
            }
        }
        Ok(())
    }

    fn draw_texts(
        &mut self,
        target: &Target<'_>,
        painted: &Painted,
        items: &[Item],
    ) -> Result<(), AtlasError> {
        let text_target = RenderTarget {
            device: target.device,
            queue: target.queue,
            view: target.view,
            viewport: target.viewport,
            scissor: target.scissor,
        };
        for item in items {
            if let Item::Text { text, origin } = item {
                let layout = painted.text(*text);
                self.text.draw(
                    &text_target,
                    &mut self.atlas,
                    layout,
                    *origin,
                    BlendParams::default(),
                )?;
            }
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

#[cfg(test)]
#[path = "painter_tests.rs"]
mod tests;
