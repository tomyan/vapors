//! What the app's painter is made with: its glyph atlas, sized for the
//! device unless the app was given a config, and its sample count, 1
//! unless the app was given 4 (`docs/plans/flat-scene-f7.md`).

use vapors_renderer::{PassFormat, Samples};
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

    /// Makes its painter draw at `samples`; a painter at 4 refuses
    /// `draw_over` and a format not guaranteed 4 samples
    /// ([`ScenePainter::new`]).
    #[must_use]
    pub fn with_samples(mut self, samples: Samples) -> Self {
        self.set_samples(samples);
        self
    }

    /// As [`ElementsApp::with_samples`], in place.
    ///
    /// # Panics
    ///
    /// Once its painter is made (the first frame's start): a painter is
    /// made for one sample count.
    pub fn set_samples(&mut self, samples: Samples) {
        assert!(
            self.painter.is_none(),
            "set_samples after the painter was made: a painter draws at one sample count; set \
             it before the first frame (elements_app_atlas.rs); reproduce with `cargo xtask test`"
        );
        self.samples = samples;
    }

    /// A painter for `device` drawing into `format` at the samples
    /// given, with the atlas given.
    pub(super) fn make_painter(
        &self,
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
    ) -> Result<ScenePainter, NewPainterError> {
        let format = PassFormat {
            format,
            samples: self.samples,
        };
        match self.atlas {
            Some(atlas) => ScenePainter::with_atlas(device, format, atlas),
            None => ScenePainter::new(device, format),
        }
    }
}
