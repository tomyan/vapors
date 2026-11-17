//! The frame's depth attachment and the 2D pipelines' depth variants
//! (`docs/plans/flat-scene.md`, F8a). A pass has a `Depth32Float`
//! attachment only when the frame paints a scene; a pipeline's depth
//! state must match its pass's, so every 2D pipeline is built twice at
//! the painter's making, without and with depth, and the target picks
//! (L78: every page, until F10's `PaintUses` says which need them). 2D
//! draws with depth test `Always` and no depth write: they neither read
//! nor change the scene's depth.

use crate::Target;

/// The depth format of every pass and pipeline with depth: the frame's
/// attachment and `SceneRenderer`'s (vapors-scene3d-gpu uses this one).
pub const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

/// Whether a pass has a depth attachment, and so which variant of a
/// pipeline it takes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Depth {
    /// No depth attachment: a frame that paints no scene.
    None,
    /// The frame's depth attachment.
    Attached,
}

/// A 2D pipeline's depth state for a pass with `depth`.
#[must_use]
pub fn depth_2d(depth: Depth) -> Option<wgpu::DepthStencilState> {
    (depth == Depth::Attached).then(|| wgpu::DepthStencilState {
        format: DEPTH_FORMAT,
        depth_write_enabled: Some(false),
        depth_compare: Some(wgpu::CompareFunction::Always),
        stencil: wgpu::StencilState::default(),
        bias: wgpu::DepthBiasState::default(),
    })
}

/// One 2D pipeline in both variants.
#[derive(Debug)]
pub struct DepthVariants {
    flat: wgpu::RenderPipeline,
    deep: wgpu::RenderPipeline,
}

impl DepthVariants {
    /// Both variants, `make` given the pass's depth.
    pub fn new(make: impl Fn(Depth) -> wgpu::RenderPipeline) -> Self {
        Self {
            flat: make(Depth::None),
            deep: make(Depth::Attached),
        }
    }

    /// The variant for `target`'s pass.
    #[must_use]
    pub fn for_target(&self, target: &Target<'_>) -> &wgpu::RenderPipeline {
        if target.depth.is_some() {
            &self.deep
        } else {
            &self.flat
        }
    }
}
