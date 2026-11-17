//! What a 2D pipeline's render pass is: its colour format and its
//! sample count (`docs/plans/flat-scene-f7.md`). A pipeline is made for
//! one; a painter for a page with a scene is 4x, every other 1x.

/// Samples per pixel of a pass's colour attachment: 1, or 4, which
/// WebGPU guarantees for the formats the painter draws to.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Samples {
    /// One sample: every page without a scene.
    One,
    /// Four samples, resolved into the target at each pass's end.
    Four,
}

impl Samples {
    /// The count wgpu takes.
    #[must_use]
    pub fn count(self) -> u32 {
        match self {
            Self::One => 1,
            Self::Four => 4,
        }
    }

    /// The multisample state of a pipeline drawing into a pass of this
    /// many samples.
    #[must_use]
    pub fn state(self) -> wgpu::MultisampleState {
        wgpu::MultisampleState {
            count: self.count(),
            ..wgpu::MultisampleState::default()
        }
    }
}

/// A pass's colour format and sample count.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PassFormat {
    /// The colour attachment's format.
    pub format: wgpu::TextureFormat,
    /// Its samples per pixel.
    pub samples: Samples,
}

/// A format alone is a pass of one sample: what every pipeline was made
/// for before F7.
impl From<wgpu::TextureFormat> for PassFormat {
    fn from(format: wgpu::TextureFormat) -> Self {
        Self {
            format,
            samples: Samples::One,
        }
    }
}
