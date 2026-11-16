//! The glyph atlas the app's painter is made with: sized for the device
//! unless the app was given a config.

use vapors_text::AtlasConfig;

use super::ElementsApp;
use crate::painter::{NewPainterError, ScenePainter};

impl ElementsApp {
    /// Makes its painter's glyph atlas from `atlas`, in place of
    /// `AtlasConfig::for_max_texture_side` of the device's
    /// `max_texture_dimension_2d`; the first frame refuses a config the
    /// device cannot hold, naming it.
    #[must_use]
    pub fn with_atlas(mut self, atlas: AtlasConfig) -> Self {
        self.atlas = Some(atlas);
        self
    }

    /// A painter for `device` drawing into `format`, with the atlas given.
    pub(super) fn make_painter(
        &self,
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
    ) -> Result<ScenePainter, NewPainterError> {
        match self.atlas {
            Some(atlas) => ScenePainter::with_atlas(device, format, atlas),
            None => ScenePainter::new(device, format),
        }
    }
}
