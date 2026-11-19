//! The lights uniform: the scene's ambient level and up to
//! [`MAX_LIGHTS`] lights in one buffer, written only when the bytes the
//! scene now gives differ from the ones last written. No pipeline reads
//! it yet (`docs/plans/lighting.md`, LT3).
//!
//! Layout, all colours linear light times intensity:
//! - 16 bytes: ambient `rgb`, 0.
//! - 16 bytes: the light count as a `u32`, then 3 zero words.
//! - per light, 48 bytes in three `vec4`s: colour `rgb` and the kind
//!   (`u32`, 0 directional, 1 point); position `xyz` and range (0 for a
//!   directional); direction `xyz` (the world -Z, unit length; zeros
//!   for a point) and 0.
//!
//! Unused lights are zeros.

use crate::FrameError;
use vapors_app::SurfaceFrame;
use vapors_scene3d::{Light, MAX_LIGHTS, NodeId, Scene, Vec3};

/// The uniform's size: 32 bytes of header and [`MAX_LIGHTS`] of 48.
const WORDS: usize = 8 + MAX_LIGHTS * 12;
pub(crate) const LIGHTS_BYTES: u64 = WORDS as u64 * 4;

pub(crate) struct LightsUniform {
    buffer: wgpu::Buffer,
    /// What the buffer holds: zeros at first, as the device makes it,
    /// which is what a scene without lights gives.
    written: Vec<u8>,
}

impl LightsUniform {
    pub(crate) fn new(device: &wgpu::Device) -> Self {
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("scene3d lights"),
            size: LIGHTS_BYTES,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let written = vec![0; WORDS * 4];
        Self { buffer, written }
    }

    /// Writes the scene's lights and ambient through `frame.work`
    /// unless they are what was last written. `scene` must be updated.
    ///
    /// # Errors
    ///
    /// [`FrameError::LightDegenerate`] for a light with no usable
    /// position or direction.
    pub(crate) fn write(
        &mut self,
        frame: &SurfaceFrame<'_>,
        scene: &Scene,
    ) -> Result<(), FrameError> {
        let bytes = lights_bytes(scene)?;
        if bytes != self.written {
            frame
                .work
                .write_buffer(frame.queue, &self.buffer, 0, &bytes);
            self.written = bytes;
        }
        Ok(())
    }
}

/// The uniform's [`LIGHTS_BYTES`] for `scene`.
pub(crate) fn lights_bytes(scene: &Scene) -> Result<Vec<u8>, FrameError> {
    let lights = scene.lights();
    let [r, g, b] = scene.ambient().linear();
    let count = u32::try_from(lights.len()).expect("at most 8 lights");
    let mut words = vec![r.to_bits(), g.to_bits(), b.to_bits(), 0, count, 0, 0, 0];
    for (node, light) in lights {
        words.extend(light_words(scene, node, light)?);
    }
    words.resize(WORDS, 0);
    Ok(words.into_iter().flat_map(u32::to_le_bytes).collect())
}

/// One light's twelve words.
fn light_words(scene: &Scene, node: NodeId, light: Light) -> Result<[u32; 12], FrameError> {
    let world = scene.world(node)?;
    let position = world.w_axis.truncate();
    let direction = world.transform_vector3(-Vec3::Z).normalize_or_zero();
    if !position.is_finite() || direction == Vec3::ZERO {
        return Err(FrameError::LightDegenerate(node));
    }
    let (colour, intensity, kind, range) = match light {
        Light::Directional { colour, intensity } => (colour, intensity, 0, 0.0),
        Light::Point {
            colour,
            intensity,
            range,
        } => (colour, intensity, 1, range),
    };
    let [r, g, b] = colour.linear().map(|c| c * intensity);
    let (position, direction) = if kind == 1 {
        (position, Vec3::ZERO)
    } else {
        (Vec3::ZERO, direction)
    };
    let f = f32::to_bits;
    Ok([
        f(r),
        f(g),
        f(b),
        kind,
        f(position.x),
        f(position.y),
        f(position.z),
        f(range),
        f(direction.x),
        f(direction.y),
        f(direction.z),
        0,
    ])
}

#[cfg(test)]
#[path = "lights_tests.rs"]
mod tests;
