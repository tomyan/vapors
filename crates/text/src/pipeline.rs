//! The wgpu side of text: one pipeline drawing atlas quads with the
//! gamma and contrast shader, the atlas texture it samples (its view
//! kept with it), the buffers it draws from (each frame's, and the
//! kept one) and the bind group over them, kept between frames. Create
//! once per target format.

use vapors_renderer::{FrameArena, Retained, UniformSlots, WorkCounter};

use crate::GlyphAtlas;
use crate::atlas_texture::AtlasTexture;
use crate::pipeline_gpu::{bind_group_layout, render_pipeline};

/// The two blend parameters. See `docs/text.md` for how they were chosen.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BlendParams {
    /// Exponent taking encoded channels to linear light.
    pub gamma: f32,
    /// Coverage boost for dark text on light backgrounds; 0 is none.
    pub contrast: f32,
}

impl Default for BlendParams {
    /// Chromium's Skia build on Linux uses 1.2 and 0.2; the comparison
    /// page in `docs/text.md` is where these were confirmed or moved.
    fn default() -> Self {
        Self {
            gamma: 1.2,
            contrast: 0.2,
        }
    }
}

/// Floats per vertex: position 2, uv 2, colour 4.
pub(crate) const FLOATS_PER_VERTEX: usize = 8;
pub(crate) const VERTEX_SIZE: u64 = (FLOATS_PER_VERTEX * std::mem::size_of::<f32>()) as u64;

/// Draws laid-out text into a render target.
pub struct TextPipeline {
    pub(crate) pipeline: wgpu::RenderPipeline,
    pub(crate) layout: wgpu::BindGroupLayout,
    pub(crate) sampler: wgpu::Sampler,
    /// The blend parameters, one slot per distinct pair drawn with.
    pub(crate) params: UniformSlots,
    /// The atlas texture, its view and the bind group over them.
    pub(crate) atlas_texture: AtlasTexture,
    pub(crate) vertices: FrameArena,
    /// Vertex bytes kept between frames (`draw_kept.rs`).
    pub(crate) kept: Retained,
    pub(crate) work: WorkCounter,
}

impl TextPipeline {
    /// A pipeline for targets of `format`, with an atlas texture matching
    /// `atlas`'s coverage page.
    #[must_use]
    pub fn new(device: &wgpu::Device, format: wgpu::TextureFormat, atlas: &GlyphAtlas) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("text"),
            source: wgpu::ShaderSource::Wgsl(include_str!("text.wgsl").into()),
        });
        let layout = bind_group_layout(device);
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("text"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        Self {
            pipeline: render_pipeline(device, &pipeline_layout, &shader, format),
            layout,
            sampler: device.create_sampler(&wgpu::SamplerDescriptor {
                label: Some("text nearest"),
                mag_filter: wgpu::FilterMode::Nearest,
                min_filter: wgpu::FilterMode::Nearest,
                ..Default::default()
            }),
            params: UniformSlots::new(device, "text params"),
            atlas_texture: AtlasTexture::new(device, atlas.coverage_page().size()),
            vertices: FrameArena::new(
                device,
                "text quads",
                wgpu::BufferUsages::VERTEX,
                VERTEX_SIZE,
            ),
            kept: Retained::new(device, "text kept", wgpu::BufferUsages::VERTEX),
            work: WorkCounter::default(),
        }
    }

    /// The work its draws issued since the last take, the atlas page's
    /// uploads included; the count restarts.
    pub fn take_work(&self) -> vapors_renderer::GpuWork {
        self.work.take()
    }

    /// The bind groups kept: one once this pipeline has drawn.
    #[must_use]
    pub fn bind_groups_kept(&self) -> u64 {
        self.atlas_texture.bind_groups_kept()
    }
}

/// The bind group over the atlas view, the sampler and the params
/// slots' buffer, counted in `work`: made once per buffer by
/// [`TextPipeline::draw`], which keeps it.
pub(crate) fn bind_group(
    device: &wgpu::Device,
    work: &WorkCounter,
    layout: &wgpu::BindGroupLayout,
    view: &wgpu::TextureView,
    sampler: &wgpu::Sampler,
    params: &UniformSlots,
) -> wgpu::BindGroup {
    work.create_bind_group(
        device,
        &wgpu::BindGroupDescriptor {
            label: Some("text"),
            layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: params.binding(),
                },
            ],
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Chromium's Skia build on Linux: `SK_GAMMA_EXPONENT=1.2` and
    /// `SK_GAMMA_CONTRAST=0.2` in chromium's `skia/BUILD.gn`; confirmed
    /// on the comparison page (docs/text.md).
    #[test]
    fn default_parameters_are_chromiums_linux_values() {
        // When
        let params = BlendParams::default();

        // Then
        assert_eq!(params.gamma, 1.2);
        assert_eq!(params.contrast, 0.2);
    }
}
