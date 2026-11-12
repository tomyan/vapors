//! Drawing a run of the scene's quads: element backgrounds and solid
//! borders with circular corner radii, anti-aliased by the shader's
//! signed distance to the rounded edges.

use vapors_scene::{Color, Quad};

use crate::GpuWork;
use crate::Target;
use crate::instanced::{Instanced, float_bytes};

/// A pipeline that draws quads. Create once per target format.
pub struct QuadPipeline(Instanced);

impl QuadPipeline {
    /// Build the pipeline for targets of `format`.
    #[must_use]
    pub fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        Self(Instanced::new(
            device,
            format,
            "quads",
            include_str!("quads.wgsl"),
            8,
        ))
    }

    /// Draw `quads` into `target`, later quads over earlier ones, in one
    /// draw call. Nothing is recorded for an empty slice.
    pub fn draw(&mut self, target: &Target<'_>, quads: &[Quad]) {
        let bytes = float_bytes(quads.iter().flat_map(instance));
        self.0.draw(target, &bytes);
    }

    /// The bind groups kept: one once this pipeline has drawn.
    #[must_use]
    pub fn bind_groups_kept(&self) -> u64 {
        self.0.bind_groups_kept()
    }

    /// The work its draws issued since the last take; the count restarts.
    pub fn take_work(&self) -> GpuWork {
        self.0.work.take()
    }
}

/// The quad's eight four-float attributes, as `quads.wgsl` reads them.
fn instance(quad: &Quad) -> Vec<f32> {
    let r = quad.rect;
    let mut floats = vec![r.left, r.top, r.right, r.bottom];
    floats.extend(rgba(quad.background));
    floats.extend(quad.border);
    for color in quad.border_colors {
        floats.extend(rgba(color));
    }
    floats.extend(quad.radii);
    floats
}

pub(crate) fn rgba(color: Color) -> [f32; 4] {
    [color.r, color.g, color.b, color.a]
}
