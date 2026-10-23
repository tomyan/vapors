//! Drawing a run of the scene's outer box shadows: a rounded rectangle
//! blurred by a Gaussian, in closed form along one axis and four
//! samples along the other, drawn outside the box that casts it.

use vapors_scene::Shadow;

use crate::instanced::{Instanced, Target, float_bytes};
use crate::quads::rgba;

/// Three standard deviations: past this the Gaussian is under 0.3% of
/// its peak, so the quad drawn for a shadow stops there.
const REACH: f32 = 3.0;

/// A pipeline that draws shadows. Create once per target format.
pub struct ShadowPipeline(Instanced);

impl ShadowPipeline {
    /// Build the pipeline for targets of `format`.
    #[must_use]
    pub fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        Self(Instanced::new(
            device,
            format,
            "shadows",
            include_str!("shadows.wgsl"),
            7,
        ))
    }

    /// Draw `shadows` into `target`, in one submit.
    pub fn draw(&mut self, target: &Target<'_>, shadows: &[Shadow]) {
        let bytes = float_bytes(shadows.iter().flat_map(instance));
        self.0.draw(target, &bytes);
    }
}

/// The shadow's seven four-float attributes, as `shadows.wgsl` reads them:
/// the quad to draw, the shape, its radii, the caster, its radii, and
/// the colour, and the standard deviation.
fn instance(shadow: &Shadow) -> Vec<f32> {
    let sigma = shadow.blur / 2.0;
    let reach = sigma * REACH;
    let (r, c) = (shadow.rect, shadow.caster);
    let mut floats = vec![
        r.left - reach,
        r.top - reach,
        r.right + reach,
        r.bottom + reach,
    ];
    floats.extend([r.left, r.top, r.right, r.bottom]);
    floats.extend(shadow.radii);
    floats.extend([c.left, c.top, c.right, c.bottom]);
    floats.extend(shadow.caster_radii);
    let [red, green, blue, alpha] = rgba(shadow.color);
    floats.extend([red, green, blue, alpha]);
    floats.push(sigma);
    floats.extend([0.0; 3]);
    floats
}
